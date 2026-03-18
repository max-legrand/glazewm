use objc2::rc::Retained;
use objc2_app_kit::NSWorkspace;
use tokio::sync::mpsc;

use crate::{
  platform_impl::{
    NotificationCenter, NotificationEvent, NotificationName,
    NotificationObserver,
  },
  Dispatcher, SessionEvent, ThreadBound,
};

/// Platform-specific implementation of [`SessionListener`].
pub(crate) struct SessionListener {
  /// Notification observer bound to the main thread.
  observer: Option<ThreadBound<Retained<NotificationObserver>>>,
}

impl SessionListener {
  /// Creates an instance of `SessionListener`.
  pub(crate) fn new(
    event_tx: mpsc::UnboundedSender<SessionEvent>,
    dispatcher: &Dispatcher,
  ) -> crate::Result<Self> {
    let dispatcher_clone = dispatcher.clone();
    let observer = dispatcher.dispatch_sync(move || {
      Self::add_observer(event_tx, dispatcher_clone)
    })?;

    Ok(Self {
      observer: Some(observer),
    })
  }

  /// Terminates the session listener.
  #[allow(clippy::unnecessary_wraps)]
  pub(crate) fn terminate(&mut self) -> crate::Result<()> {
    self.observer.take();
    Ok(())
  }

  /// Registers the notification observer on the main thread.
  fn add_observer(
    event_tx: mpsc::UnboundedSender<SessionEvent>,
    dispatcher: Dispatcher,
  ) -> ThreadBound<Retained<NotificationObserver>> {
    let (observer, mut events_rx) = NotificationObserver::new();
    let mut workspace_center = NotificationCenter::workspace_center();
    let workspace = NSWorkspace::sharedWorkspace();

    // Register for session and power state notifications via the
    // workspace notification center.
    for notification in [
      NotificationName::WorkspaceSessionDidResignActive,
      NotificationName::WorkspaceSessionDidBecomeActive,
      NotificationName::WorkspaceWillSleep,
      NotificationName::WorkspaceDidWake,
      NotificationName::WorkspaceScreensDidSleep,
      NotificationName::WorkspaceScreensDidWake,
    ] {
      unsafe {
        workspace_center.add_observer(
          notification,
          &observer,
          Some(&workspace),
        );
      }
    }

    // Register for screen lock/unlock via the distributed notification
    // center. These are the most reliable notifications for detecting
    // when the user locks/unlocks the screen.
    let mut dist_center = NotificationCenter::distributed_center();
    for notification in [
      NotificationName::ScreenIsLocked,
      NotificationName::ScreenIsUnlocked,
    ] {
      unsafe {
        dist_center.add_observer(notification, &observer, None);
      }
    }

    std::thread::spawn(move || {
      while let Some(event) = events_rx.blocking_recv() {
        let session_event = match event {
          NotificationEvent::WorkspaceSessionDidResignActive
          | NotificationEvent::ScreenIsLocked => {
            Some(SessionEvent::Locked)
          }
          NotificationEvent::WorkspaceSessionDidBecomeActive
          | NotificationEvent::WorkspaceScreensDidWake
          | NotificationEvent::ScreenIsUnlocked => {
            Some(SessionEvent::Unlocked)
          }
          NotificationEvent::WorkspaceWillSleep
          | NotificationEvent::WorkspaceScreensDidSleep => {
            Some(SessionEvent::Suspending)
          }
          NotificationEvent::WorkspaceDidWake => {
            Some(SessionEvent::Resumed)
          }
          _ => None,
        };

        if let Some(session_event) = session_event {
          if let Err(err) = event_tx.send(session_event) {
            tracing::warn!("Failed to send session event: {}", err);
            break;
          }
        }
      }

      tracing::debug!("Session listener thread exited.");
    });

    ThreadBound::new(observer, dispatcher)
  }
}

impl Drop for SessionListener {
  fn drop(&mut self) {
    let _ = self.terminate();
  }
}
