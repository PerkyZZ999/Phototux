//! Background paint worker (ADR-007).

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};

use phototux_engine::{
    BrushParams, EngineCommand, EngineEvent, LayerId, PaintTarget, StrokeBuilder, StrokeJournal,
};

struct WorkerState {
    brush: BrushParams,
    stroke: Option<StrokeBuilder>,
    layer: Option<LayerId>,
    target: Option<PaintTarget>,
    first_input_ms: Option<f64>,
    dabs_since_composite: u32,
    journal: StrokeJournal,
}

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
        stroke: None,
        layer: None,
        target: None,
        first_input_ms: None,
        dabs_since_composite: 0,
        journal: StrokeJournal::default(),
    };

    while let Ok(cmd) = rx.recv() {
        match cmd {
            EngineCommand::Shutdown => break,
            EngineCommand::SetBrush(p) => apply_set_brush(&mut st, p),
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
    st.first_input_ms = Some(t_ms);
    st.dabs_since_composite = 0;
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
        if let Err(e) = super::document_gpu::stamp_dabs(layer, target, &[], st.brush, None, true) {
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
    st.dabs_since_composite = 0;
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
    st.dabs_since_composite += dabs.len() as u32;
    // Coalesce: composite every few dabs for latency/FPS balance
    let recomposite = force_composite || st.dabs_since_composite >= 4;
    let t0 = if force_composite {
        st.first_input_ms
    } else {
        None
    };
    match super::document_gpu::stamp_dabs(layer, target, dabs, st.brush, t0, recomposite) {
        Ok(ms) => {
            if recomposite {
                st.dabs_since_composite = 0;
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
}
