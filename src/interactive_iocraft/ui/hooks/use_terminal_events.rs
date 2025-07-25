//! Terminal event handling hook

use iocraft::prelude::*;
use futures::stream;
use std::sync::mpsc;
use std::pin::Pin;

/// Hook that provides access to terminal events
pub fn use_terminal_events(hooks: &mut Hooks) -> Pin<std::boxed::Box<dyn futures::Stream<Item = TerminalEvent> + Send + 'static>> {
    let (tx, rx) = mpsc::channel();
    
    hooks.use_terminal_events(move |event| {
        let _ = tx.send(event);
    });
    
    // Convert the receiver to a stream and pin it
    std::boxed::Box::pin(stream::unfold(rx, |rx| async move {
        rx.recv().ok().map(|event| (event, rx))
    }))
}

/// Convenient key event matcher
pub fn is_quit_key(key: &iocraft::KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            ..
        } | KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
    )
}

pub fn is_escape_key(key: &KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Esc,
            ..
        }
    )
}

pub fn is_enter_key(key: &KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Enter,
            ..
        }
    )
}