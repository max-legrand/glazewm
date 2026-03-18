use tokio::sync::mpsc;

use crate::{platform_impl, Dispatcher};

/// Events emitted by the session listener.
#[derive(Clone, Debug)]
pub enum SessionEvent {
  /// The session was locked (e.g. screen lock or user switch away).
  Locked,

  /// The session was unlocked (e.g. user logged back in).
  Unlocked,

  /// The system is entering sleep/hibernation.
  Suspending,

  /// The system is resuming from sleep/hibernation.
  Resumed,
}

/// A listener for system session state changes.
///
/// Detects session lock/unlock events and system sleep/resume events.
///
/// # Platform-specific
///
/// - **Windows**: Listens for `WM_WTSSESSION_CHANGE` (lock/unlock) and
///   `WM_POWERBROADCAST` (sleep/resume) messages.
/// - **macOS**: Listens for `NSWorkspaceSessionDidBecomeActive`/
///   `NSWorkspaceSessionDidResignActive` (session switch) and
///   `NSWorkspaceDidWake`/`NSWorkspaceWillSleep` (sleep/wake)
///   notifications.
pub struct SessionListener {
  event_rx: mpsc::UnboundedReceiver<SessionEvent>,

  /// Inner platform-specific session listener.
  inner: platform_impl::SessionListener,
}

impl SessionListener {
  /// Creates a new [`SessionListener`].
  pub fn new(dispatcher: &Dispatcher) -> crate::Result<Self> {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let inner =
      platform_impl::SessionListener::new(event_tx, dispatcher)?;
    Ok(Self { event_rx, inner })
  }

  /// Returns when the next session event is detected.
  ///
  /// Returns `None` if the channel has been closed.
  pub async fn next_event(&mut self) -> Option<SessionEvent> {
    self.event_rx.recv().await
  }

  /// Terminates the session listener.
  pub fn terminate(&mut self) -> crate::Result<()> {
    self.inner.terminate()
  }
}
