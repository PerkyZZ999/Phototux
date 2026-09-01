//! Background paint worker (ADR-007).

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use phototux_engine::{
    BrushParams, EngineCommand, EngineEvent, LayerId, PaintTarget, StrokeBuilder, StrokeJournal,
};

struct WorkerState {
    brush: BrushParams,
    /// Where the clone stamp reads, relative to where it writes.
    ///
    /// Set once per stroke from the anchor the user alt-clicked, so the copy
    /// stays aligned with the original instead of following the cursor.
    clone_offset: (i32, i32),
    /// The point the clone stamp was anchored to, if any.
    clone_anchor: Option<(f32, f32)>,
    stroke: Option<StrokeBuilder>,
    layer: Option<LayerId>,
    target: Option<PaintTarget>,
    first_input_ms: Option<f64>,
    /// Dabs are stamped but not yet composited, so the canvas is behind.
    pending_dabs: bool,
    /// When the last composite went out, for pacing the next one.
    last_composite: Option<Instant>,
    journal: StrokeJournal,
}

/// Shortest gap between mid-stroke composites.
///
/// Dab rate is pointer speed over spacing, which has nothing to do with the
/// refresh rate: a small brush moved quickly used to composite several times
/// per displayed frame, and a large brush moved slowly went seconds without
/// compositing at all. Pacing on elapsed time bounds both. 8 ms covers a 120 Hz
/// display without compositing twice for one frame at 60 Hz.
const MIN_COMPOSITE_GAP: Duration = Duration::from_millis(8);

/// Handle to enqueue paint commands from the UI thread.
pub struct PaintWorker {
    tx: Option<Sender<EngineCommand>>,
    rx_ev: Receiver<EngineEvent>,
    join: Option<JoinHandle<()>>,
    start_error: Option<String>,
}

impl PaintWorker {
    /// Spawn the paint worker thread.
    ///
    /// # Errors
    /// Returns an error when the OS rejects thread creation. The returned worker still
    /// exists with `send` failing so UI construction can report the failure.
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel::<EngineCommand>();
        let (tx_ev, rx_ev) = mpsc::channel::<EngineEvent>();
        match thread::Builder::new()
            .name("phototux-paint".into())
            .spawn(move || worker_loop(rx, tx_ev))
        {
            Ok(join) => Self {
                tx: Some(tx),
                rx_ev,
                join: Some(join),
                start_error: None,
            },
            Err(error) => Self {
                tx: None,
                rx_ev,
                join: None,
                start_error: Some(format!("failed to spawn paint worker: {error}")),
            },
        }
    }

    pub fn start_error(&self) -> Option<&str> {
        self.start_error.as_deref()
    }

    /// Enqueue a paint command.
    ///
    /// # Errors
    /// Returns an error when the worker failed to start or has shut down.
    pub fn send(&self, cmd: EngineCommand) -> Result<(), String> {
        let Some(tx) = self.tx.as_ref() else {
            return Err(self
                .start_error
                .clone()
                .unwrap_or_else(|| "paint worker unavailable".to_owned()));
        };
        tx.send(cmd).map_err(|_| "paint worker stopped".to_owned())
    }

    pub fn poll_events(&self) -> Vec<EngineEvent> {
        let mut out = Vec::new();
        loop {
            match self.rx_ev.try_recv() {
                Ok(e) => out.push(e),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        out
    }
}

impl Drop for PaintWorker {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(EngineCommand::Shutdown);
        }
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

fn worker_loop(rx: Receiver<EngineCommand>, tx_ev: Sender<EngineEvent>) {
    let mut st = WorkerState {
        brush: BrushParams::default(),
        clone_offset: (0, 0),
        clone_anchor: None,
        stroke: None,
        layer: None,
        target: None,
        first_input_ms: None,
        pending_dabs: false,
        last_composite: None,
        journal: StrokeJournal::default(),
    };

    loop {
        // Block indefinitely when nothing is owed to the canvas; wait only as
        // long as the pacing gap when dabs are stamped but not yet shown, so a
        // stroke that pauses still lands its last dabs.
        let cmd = if st.pending_dabs {
            match rx.recv_timeout(MIN_COMPOSITE_GAP) {
                Ok(cmd) => cmd,
                Err(RecvTimeoutError::Timeout) => {
                    flush_pending(&mut st, &tx_ev);
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match rx.recv() {
                Ok(cmd) => cmd,
                Err(_) => break,
            }
        };
        match cmd {
            EngineCommand::Shutdown => break,
            EngineCommand::SetBrush(p) => apply_set_brush(&mut st, p),
            EngineCommand::SetCloneAnchor { x, y } => st.clone_anchor = Some((x, y)),
            EngineCommand::BeginStroke {
                layer,
                target,
                x,
                y,
                pressure,
                t_ms,
            } => handle_begin_stroke(&mut st, &tx_ev, layer, target, x, y, pressure, t_ms),
            EngineCommand::StrokePoint {
                x,
                y,
                pressure,
                t_ms,
            } => handle_stroke_point(&mut st, &tx_ev, x, y, pressure, t_ms),
            EngineCommand::EndStroke => handle_end_stroke(&mut st, &tx_ev),
        }
    }
}

fn apply_set_brush(st: &mut WorkerState, p: phototux_engine::BrushParams) {
    st.brush = p.clamped();
    if let Some(s) = st.stroke.as_mut() {
        s.set_params(st.brush);
    }
}

fn handle_begin_stroke(
    st: &mut WorkerState,
    tx_ev: &Sender<EngineEvent>,
    layer: phototux_engine::LayerId,
    target: phototux_engine::PaintTarget,
    x: f32,
    y: f32,
    pressure: f32,
    t_ms: f64,
) {
    if let Err(e) = super::document_gpu::begin_stroke(layer, target) {
        let _ = tx_ev.send(EngineEvent::Error(e));
        return;
    }
    st.layer = Some(layer);
    st.target = Some(target);
    // Fix the clone offset for the whole stroke: an offset recomputed per dab
    // would make the copy chase the cursor instead of staying aligned with the
    // original, which is the entire point of an aligned clone.
    st.clone_offset = st.clone_anchor.map_or((0, 0), |(ax, ay)| {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "document coordinates, bounded by the canvas"
        )]
        let offset = ((ax - x) as i32, (ay - y) as i32);
        offset
    });
    st.first_input_ms = Some(t_ms);
    st.pending_dabs = false;
    st.last_composite = None;
    st.journal
        .begin(layer, target, st.brush, x, y, pressure, t_ms);
    let mut builder = phototux_engine::StrokeBuilder::new(st.brush);
    let dabs = builder.begin(x, y, pressure);
    st.journal.push_dabs(&dabs);
    st.stroke = Some(builder);
    apply_dabs(st, &dabs, true, tx_ev);
}

fn handle_stroke_point(
    st: &mut WorkerState,
    tx_ev: &Sender<EngineEvent>,
    x: f32,
    y: f32,
    pressure: f32,
    t_ms: f64,
) {
    if st.first_input_ms.is_none() {
        st.first_input_ms = Some(t_ms);
    }
    st.journal.sample(x, y, pressure, t_ms);
    let Some(builder) = st.stroke.as_mut() else {
        return;
    };
    let dabs = builder.move_to(x, y, pressure);
    if !dabs.is_empty() {
        st.journal.push_dabs(&dabs);
        apply_dabs(st, &dabs, false, tx_ev);
    }
}

fn handle_end_stroke(st: &mut WorkerState, tx_ev: &Sender<EngineEvent>) {
    if let Some(builder) = st.stroke.as_mut() {
        builder.end();
    }
    st.stroke = None;
    if let (Some(layer), Some(target)) = (st.layer, st.target) {
        // Flush the tail of the stroke without compositing: `end_stroke` below
        // composites unconditionally, so asking for one here paid for two full
        // canvas composites at every pen-up.
        if let Err(e) = super::document_gpu::stamp_dabs(layer, target, &[], st.brush, None, false) {
            let _ = tx_ev.send(EngineEvent::Error(e));
        }
    }
    if let Err(e) = super::document_gpu::end_stroke() {
        let _ = tx_ev.send(EngineEvent::Error(e));
    }
    let journaled = st.journal.end();
    let ms = super::document_gpu::last_composite_ms();
    let _ = tx_ev.send(EngineEvent::CompositeDone { ms });
    let lat = super::document_gpu::last_stroke_latency_ms();
    if lat > 0.0 {
        let _ = tx_ev.send(EngineEvent::StrokeLatency { ms: lat });
    }
    if let Some(entry) = journaled {
        let _ = tx_ev.send(EngineEvent::StrokeJournaled(entry));
    }
    let _ = tx_ev.send(EngineEvent::StrokeEnded);
    st.layer = None;
    st.target = None;
    st.first_input_ms = None;
    st.pending_dabs = false;
    st.last_composite = None;
}

fn apply_dabs(
    st: &mut WorkerState,
    dabs: &[phototux_engine::Dab],
    force_composite: bool,
    tx_ev: &Sender<EngineEvent>,
) {
    let Some(layer) = st.layer else {
        return;
    };
    let Some(target) = st.target else {
        return;
    };
    if !dabs.is_empty() {
        st.pending_dabs = true;
    }
    let recomposite = force_composite || composite_is_due(st);
    let t0 = if force_composite {
        st.first_input_ms
    } else {
        None
    };
    match super::document_gpu::stamp_dabs_from(
        layer,
        target,
        dabs,
        st.brush,
        st.clone_offset,
        t0,
        recomposite,
    ) {
        Ok(ms) => {
            if recomposite {
                st.pending_dabs = false;
                st.last_composite = Some(Instant::now());
                let _ = tx_ev.send(EngineEvent::CompositeDone { ms });
                if force_composite {
                    let lat = super::document_gpu::last_stroke_latency_ms();
                    if lat > 0.0 {
                        let _ = tx_ev.send(EngineEvent::StrokeLatency { ms: lat });
                    }
                }
            }
        }
        Err(e) => {
            let _ = tx_ev.send(EngineEvent::Error(e));
        }
    }
}

/// True when stamped dabs are waiting and enough time has passed to show them.
fn composite_is_due(st: &WorkerState) -> bool {
    st.pending_dabs
        && st
            .last_composite
            .is_none_or(|last| last.elapsed() >= MIN_COMPOSITE_GAP)
}

/// Composite dabs that arrived too recently to be paced out, once the stroke
/// goes quiet. Without this a stroke that pauses mid-air — pointer still down,
/// hand still — leaves its last few dabs stamped but never composited.
fn flush_pending(st: &mut WorkerState, tx_ev: &Sender<EngineEvent>) {
    if !st.pending_dabs {
        return;
    }
    let (Some(layer), Some(target)) = (st.layer, st.target) else {
        st.pending_dabs = false;
        return;
    };
    match super::document_gpu::stamp_dabs(layer, target, &[], st.brush, None, true) {
        Ok(ms) => {
            st.pending_dabs = false;
            st.last_composite = Some(Instant::now());
            let _ = tx_ev.send(EngineEvent::CompositeDone { ms });
        }
        Err(e) => {
            let _ = tx_ev.send(EngineEvent::Error(e));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phototux_engine::EngineCommand;

    #[test]
    fn start_and_send_round_trip() {
        let worker = PaintWorker::start();
        assert!(worker.start_error().is_none());
        worker
            .send(EngineCommand::SetBrush(BrushParams::default()))
            .expect("send");
        worker.send(EngineCommand::Shutdown).expect("shutdown");
    }

    fn paced_state(pending: bool, last_composite: Option<Instant>) -> WorkerState {
        WorkerState {
            brush: BrushParams::default(),
            clone_offset: (0, 0),
            clone_anchor: None,
            stroke: None,
            layer: None,
            target: None,
            first_input_ms: None,
            pending_dabs: pending,
            last_composite,
            journal: StrokeJournal::default(),
        }
    }

    /// Pacing is what keeps composite rate tied to the display rather than to
    /// how fast the pointer moves.
    #[test]
    fn composite_pacing_bounds_both_directions() {
        // Nothing stamped: never composite, however long it has been.
        let idle = paced_state(false, Some(Instant::now() - MIN_COMPOSITE_GAP * 10));
        assert!(!composite_is_due(&idle));

        // First dabs of a stroke have no previous composite to pace against.
        let first = paced_state(true, None);
        assert!(composite_is_due(&first));

        // Dabs arriving faster than the gap wait — this is the small-brush,
        // fast-stroke case that used to composite several times per frame.
        let too_soon = paced_state(true, Some(Instant::now()));
        assert!(!composite_is_due(&too_soon));

        // Once the gap has passed, pending dabs go out — this is the
        // large-brush, slow-stroke case that used to stall for seconds.
        let due = paced_state(true, Some(Instant::now() - MIN_COMPOSITE_GAP * 2));
        assert!(composite_is_due(&due));
    }

    /// The gap has to admit a 120 Hz frame without firing twice for one frame
    /// at 60 Hz, or the pacing trades one failure mode for the other.
    #[test]
    fn composite_gap_sits_between_common_refresh_rates() {
        let gap = MIN_COMPOSITE_GAP.as_secs_f32() * 1000.0;
        assert!(
            gap <= 1000.0 / 120.0,
            "gap {gap} ms starves a 120 Hz display"
        );
        assert!(
            gap > 1000.0 / 240.0,
            "gap {gap} ms paces faster than any display"
        );
    }
}
