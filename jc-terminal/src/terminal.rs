use alacritty_terminal::event::Event;
use alacritty_terminal::event::EventListener;
use alacritty_terminal::term::Config;
use alacritty_terminal::term::Term;
use parking_lot::Mutex;
use std::sync::Arc;

/// Events forwarded from the terminal to the host application.
#[derive(Debug, Clone)]
pub enum TerminalEvent {
  Wakeup,
  Bell,
  Title,
  Exit,
  ChildExit,
  PtyWrite(String),
  CursorBlinkingChange,
}

/// Forwards alacritty events to a flume channel.
pub struct EventProxy {
  tx: flume::Sender<TerminalEvent>,
}

impl EventProxy {
  pub fn new(tx: flume::Sender<TerminalEvent>) -> Self {
    Self { tx }
  }
}

impl EventListener for EventProxy {
  fn send_event(&self, event: Event) {
    let terminal_event = match event {
      Event::Wakeup => TerminalEvent::Wakeup,
      Event::Bell => TerminalEvent::Bell,
      Event::Title(_) => TerminalEvent::Title,
      Event::Exit => TerminalEvent::Exit,
      Event::ChildExit(_) => TerminalEvent::ChildExit,
      Event::PtyWrite(s) => TerminalEvent::PtyWrite(s),
      Event::CursorBlinkingChange => TerminalEvent::CursorBlinkingChange,
      _ => return,
    };
    let _ = self.tx.send(terminal_event);
  }
}

/// Dimensions type for terminal sizing.
pub struct TermDimensions {
  pub cols: usize,
  pub rows: usize,
}

impl alacritty_terminal::grid::Dimensions for TermDimensions {
  fn total_lines(&self) -> usize {
    self.rows
  }

  fn screen_lines(&self) -> usize {
    self.rows
  }

  fn columns(&self) -> usize {
    self.cols
  }
}

/// Wraps alacritty's Term behind a shared mutex.
pub struct TerminalState {
  term: Arc<Mutex<Term<EventProxy>>>,
}

impl TerminalState {
  pub fn new(cols: usize, rows: usize, event_tx: flume::Sender<TerminalEvent>) -> Self {
    let proxy = EventProxy::new(event_tx);
    let dims = TermDimensions { cols, rows };
    let config = Config::default();
    let term = Term::new(config, &dims, proxy);

    Self { term: Arc::new(Mutex::new(term)) }
  }

  /// Read-only access to the terminal.
  pub fn with_term<R>(&self, f: impl FnOnce(&Term<EventProxy>) -> R) -> R {
    let term = self.term.lock();
    f(&term)
  }

  /// Mutable access to the terminal.
  pub fn with_term_mut<R>(&self, f: impl FnOnce(&mut Term<EventProxy>) -> R) -> R {
    let mut term = self.term.lock();
    f(&mut term)
  }

  /// Resize the terminal grid and scrollback.
  pub fn resize(&self, cols: usize, rows: usize) {
    let dims = TermDimensions { cols, rows };
    let mut term = self.term.lock();
    term.resize(dims);
  }

  /// Get a clone of the Arc for use in canvas closures.
  pub fn term_handle(&self) -> Arc<Mutex<Term<EventProxy>>> {
    self.term.clone()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn new_terminal_state_dimensions() {
    let (tx, _rx) = flume::unbounded();
    let state = TerminalState::new(80, 24, tx);
    state.with_term(|term| {
      use alacritty_terminal::grid::Dimensions;
      assert_eq!(term.grid().columns(), 80);
      assert_eq!(term.grid().screen_lines(), 24);
    });
  }

  #[test]
  fn resize_changes_dimensions() {
    let (tx, _rx) = flume::unbounded();
    let state = TerminalState::new(80, 24, tx);
    state.resize(120, 40);
    state.with_term(|term| {
      use alacritty_terminal::grid::Dimensions;
      assert_eq!(term.grid().columns(), 120);
      assert_eq!(term.grid().screen_lines(), 40);
    });
  }

  #[test]
  fn with_term_mut_can_modify() {
    let (tx, _rx) = flume::unbounded();
    let state = TerminalState::new(80, 24, tx);
    state.with_term_mut(|term| {
      use alacritty_terminal::grid::Dimensions;
      assert_eq!(term.grid().columns(), 80);
    });
  }

  #[test]
  fn term_handle_shares_state() {
    let (tx, _rx) = flume::unbounded();
    let state = TerminalState::new(80, 24, tx);
    let handle = state.term_handle();
    let term = handle.lock();
    use alacritty_terminal::grid::Dimensions;
    assert_eq!(term.grid().columns(), 80);
  }

  #[test]
  fn event_proxy_sends_events() {
    let (tx, rx) = flume::unbounded();
    let proxy = EventProxy::new(tx);
    use alacritty_terminal::event::EventListener;
    proxy.send_event(alacritty_terminal::event::Event::Bell);
    let event = rx.try_recv().unwrap();
    assert!(matches!(event, TerminalEvent::Bell));
  }

  #[test]
  fn event_proxy_wakeup() {
    let (tx, rx) = flume::unbounded();
    let proxy = EventProxy::new(tx);
    use alacritty_terminal::event::EventListener;
    proxy.send_event(alacritty_terminal::event::Event::Wakeup);
    assert!(matches!(rx.try_recv().unwrap(), TerminalEvent::Wakeup));
  }

  #[test]
  fn event_proxy_exit() {
    let (tx, rx) = flume::unbounded();
    let proxy = EventProxy::new(tx);
    use alacritty_terminal::event::EventListener;
    proxy.send_event(alacritty_terminal::event::Event::Exit);
    assert!(matches!(rx.try_recv().unwrap(), TerminalEvent::Exit));
  }
}
