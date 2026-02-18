//! Event handling for the TUI application
//!
//! This module provides event types and an event handler for managing
//! keyboard input, timer ticks, and quit signals.

use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

/// Application events that drive the event loop
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Keyboard input event
    Input(KeyEvent),
    /// Timer tick for periodic updates
    Tick,
    /// Quit signal
    Quit,
}

/// Event handler that manages input events and timer ticks
pub struct EventHandler {
    receiver: Receiver<AppEvent>,
    _sender: Sender<AppEvent>,
}

impl EventHandler {
    /// Create a new event handler with channels
    ///
    /// This starts a background thread that polls for keyboard events
    /// and sends them through the channel.
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let event_sender = sender.clone();

        // Spawn a thread to handle crossterm events
        thread::spawn(move || {
            loop {
                // Poll for events with a timeout
                if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                    if let Ok(Event::Key(key)) = event::read() {
                        if event_sender.send(AppEvent::Input(key)).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Self {
            receiver,
            _sender: sender,
        }
    }

    /// Block and receive the next event
    ///
    /// This method will block until an event is received from the channel.
    pub fn next(&self) -> Result<AppEvent> {
        Ok(self.receiver.recv()?)
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_handler_creation() {
        let _handler = EventHandler::new();
        // Just verify we can create the handler without panicking
    }

    #[test]
    fn test_event_variants() {
        // Test that we can create each variant
        let _input = AppEvent::Input(KeyEvent::from(crossterm::event::KeyCode::Char('a')));
        let _tick = AppEvent::Tick;
        let _quit = AppEvent::Quit;
    }
}
