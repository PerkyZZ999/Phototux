//! Background paint worker (ADR-007).

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};

use phototux_engine::{BrushParams, EngineCommand, EngineEvent, LayerId, StrokeBuilder};

struct WorkerState {
    brush: BrushParams,
    stroke: Option<StrokeBuilder>,
    layer: Option<LayerId>,
    first_input_ms: Option<f64>,
    dabs_since_composite: u32,
}

/// Handle to enqueue paint commands from the UI thread.
pub struct PaintWorker {
    tx: Sender<EngineCommand>,
    rx_ev: Receiver<EngineEvent>,
    join: Option<JoinHandle<()>>,
}

impl PaintWorker {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel::<EngineCommand>();
        let (tx_ev, rx_ev) = mpsc::channel::<EngineEvent>();
        let join = thread::Builder::new()
            .name("phototux-paint".into())
            .spawn(move || worker_loop(rx, tx_ev))
            .expect("spawn paint worker");
        Self {
            tx,
            rx_ev,
            join: Some(join),
        }
    }

    pub fn send(&self, cmd: EngineCommand) {
        let _ = self.tx.send(cmd);
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
        let _ = self.tx.send(EngineCommand::Shutdown);
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
        first_input_ms: None,
        dabs_since_composite: 0,
    };

    while let Ok(cmd) = rx.recv() {
        match cmd {
            EngineCommand::Shutdown => break,
            EngineCommand::SetBrush(p) => {
                st.brush = p.clamped();
                if let Some(s) = st.stroke.as_mut() {
                    s.set_params(st.brush);
                }
            }
            EngineCommand::BeginStroke {
                layer,
                x,
                y,
                pressure,
                t_ms,
            } => {
                if let Err(e) = super::document_gpu::begin_stroke(layer) {
                    let _ = tx_ev.send(EngineEvent::Error(e));
                    continue;
                }
                st.layer = Some(layer);
                st.first_input_ms = Some(t_ms);
                st.dabs_since_composite = 0;
                let mut builder = StrokeBuilder::new(st.brush);
                let dabs = builder.begin(x, y, pressure);
                st.stroke = Some(builder);
                apply_dabs(&mut st, &dabs, true, &tx_ev);
            }
            EngineCommand::StrokePoint {
                x,
                y,
                pressure,
                t_ms,
            } => {
                if st.first_input_ms.is_none() {
                    st.first_input_ms = Some(t_ms);
                }
                let Some(builder) = st.stroke.as_mut() else {
                    continue;
                };
                let dabs = builder.move_to(x, y, pressure);
                if !dabs.is_empty() {
                    apply_dabs(&mut st, &dabs, false, &tx_ev);
                }
            }
            EngineCommand::EndStroke => {
                if let Some(builder) = st.stroke.as_mut() {
                    builder.end();
                }
                st.stroke = None;
                // Final composite
                if let Some(layer) = st.layer {
                    let _ = super::document_gpu::stamp_dabs(layer, &[], st.brush, None, true);
                }
                if let Err(e) = super::document_gpu::end_stroke() {
                    let _ = tx_ev.send(EngineEvent::Error(e));
                }
                let ms = super::document_gpu::last_composite_ms();
                let _ = tx_ev.send(EngineEvent::CompositeDone { ms });
                let lat = super::document_gpu::last_stroke_latency_ms();
                if lat > 0.0 {
                    let _ = tx_ev.send(EngineEvent::StrokeLatency { ms: lat });
                }
                let _ = tx_ev.send(EngineEvent::StrokeEnded);
                st.layer = None;
                st.first_input_ms = None;
                st.dabs_since_composite = 0;
            }
        }
    }
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
    st.dabs_since_composite += dabs.len() as u32;
    // Coalesce: composite every few dabs for latency/FPS balance
    let recomposite = force_composite || st.dabs_since_composite >= 4;
    let t0 = if force_composite {
        st.first_input_ms
    } else {
        None
    };
    match super::document_gpu::stamp_dabs(layer, dabs, st.brush, t0, recomposite) {
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
