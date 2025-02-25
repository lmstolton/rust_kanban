use log::error;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use super::{key::Key, mouse::Mouse, InputEvent};

/// A small event handler that wrap crossterm input and tick event. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub struct Events {
    rx: tokio::sync::mpsc::Receiver<InputEvent>,
    // Need to be kept around to prevent disposing the sender side.
    _tx: tokio::sync::mpsc::Sender<InputEvent>,
    // To stop the loop
    stop_capture: Arc<AtomicBool>,
}

impl Events {
    /// Constructs an new instance of `Events` with the default config.
    pub fn new(tick_rate: Duration) -> Events {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let stop_capture = Arc::new(AtomicBool::new(false));

        let event_tx = tx.clone();
        let event_stop_capture = stop_capture.clone();
        tokio::spawn(async move {
            loop {
                // poll for tick rate duration, if no event, sent tick event.
                if crossterm::event::poll(tick_rate).unwrap() {
                    let event = crossterm::event::read().unwrap();
                    if let crossterm::event::Event::Mouse(mouse_action) = event {
                        let mouse_action = Mouse::from(mouse_action);
                        if let Err(err) = event_tx.send(InputEvent::MouseAction(mouse_action)).await
                        {
                            error!("Oops!, {}", err);
                        }
                    } else if let crossterm::event::Event::Key(key) = event {
                        let key = Key::from(key);
                        if let Err(err) = event_tx.send(InputEvent::KeyBoardInput(key)).await {
                            error!("Oops!, {}", err);
                        }
                    }
                }
                if let Err(err) = event_tx.send(InputEvent::Tick).await {
                    error!("Oops!, {}", err);
                }
                if event_stop_capture.load(Ordering::Relaxed) {
                    break;
                }
            }
        });

        Events {
            rx,
            _tx: tx,
            stop_capture,
        }
    }

    /// Attempts to read an event.
    pub async fn next(&mut self) -> InputEvent {
        let new_event = self.rx.recv().await.unwrap_or(InputEvent::Tick);
        if new_event == InputEvent::KeyBoardInput(Key::Unknown) {
            InputEvent::Tick
        } else {
            new_event
        }
    }

    /// Close
    pub fn close(&mut self) {
        self.stop_capture.store(true, Ordering::Relaxed)
    }
}
