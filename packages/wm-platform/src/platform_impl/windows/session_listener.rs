use tokio::sync::mpsc;
use tracing::warn;
use windows::Win32::{
  Foundation::HWND,
  System::RemoteDesktop::{
    WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
    NOTIFY_FOR_THIS_SESSION,
  },
  UI::WindowsAndMessaging::{
    PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMESUSPEND, PBT_APMSUSPEND,
    WM_POWERBROADCAST, WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK,
    WTS_SESSION_UNLOCK,
  },
};

use crate::{Dispatcher, DispatcherExtWindows, SessionEvent};

/// Listens for Windows session lock/unlock and sleep/resume events.
pub(crate) struct SessionListener {
  callback_id: Option<usize>,
  message_window_handle: isize,
  dispatcher: Dispatcher,
}

impl SessionListener {
  /// Creates a new `SessionListener`.
  ///
  /// Registers for WTS session notifications and power broadcast
  /// messages on the event loop's message window.
  pub(crate) fn new(
    event_tx: mpsc::UnboundedSender<SessionEvent>,
    dispatcher: &Dispatcher,
  ) -> crate::Result<Self> {
    let message_window_handle = dispatcher.message_window_handle();

    // Register for session change notifications on the message window.
    // This must be done on the event loop thread since the HWND belongs
    // to that thread.
    dispatcher.dispatch_async({
      let hwnd = message_window_handle;
      move || {
        let result = unsafe {
          WTSRegisterSessionNotification(
            HWND(hwnd),
            NOTIFY_FOR_THIS_SESSION,
          )
        };

        if let Err(err) = result {
          warn!("Failed to register for session notifications: {}", err);
        }
      }
    })?;

    let callback_id = dispatcher.register_wndproc_callback(Box::new(
      move |_hwnd, message, wparam, _lparam| {
        match message {
          WM_WTSSESSION_CHANGE => {
            #[allow(clippy::cast_possible_truncation)]
            let event = match wparam as u32 {
              WTS_SESSION_LOCK => Some(SessionEvent::Locked),
              WTS_SESSION_UNLOCK => Some(SessionEvent::Unlocked),
              _ => None,
            };

            if let Some(event) = event {
              let _ = event_tx.send(event);
            }

            Some(0)
          }
          WM_POWERBROADCAST => {
            #[allow(clippy::cast_possible_truncation)]
            let event = match wparam as u32 {
              PBT_APMSUSPEND => Some(SessionEvent::Suspending),
              PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND => {
                Some(SessionEvent::Resumed)
              }
              _ => None,
            };

            if let Some(event) = event {
              let _ = event_tx.send(event);
            }

            // Don't consume the message - let the display listener
            // also handle WM_POWERBROADCAST for its suspend tracking.
            None
          }
          _ => None,
        }
      },
    ))?;

    Ok(Self {
      callback_id: Some(callback_id),
      message_window_handle,
      dispatcher: dispatcher.clone(),
    })
  }

  /// Terminates the session listener.
  pub(crate) fn terminate(&mut self) -> crate::Result<()> {
    if let Some(id) = self.callback_id.take() {
      self.dispatcher.deregister_wndproc_callback(id)?;

      let hwnd = self.message_window_handle;
      self.dispatcher.dispatch_async(move || {
        let _ = unsafe { WTSUnRegisterSessionNotification(HWND(hwnd)) };
      })?;
    }

    Ok(())
  }
}

impl Drop for SessionListener {
  fn drop(&mut self) {
    if let Err(err) = self.terminate() {
      warn!("Failed to terminate session listener: {}", err);
    }
  }
}
