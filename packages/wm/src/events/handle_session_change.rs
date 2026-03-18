use wm_common::WmEvent;
use wm_platform::SessionEvent;

use crate::{
  commands::window::manage_window,
  events::handle_display_settings_changed, user_config::UserConfig,
  wm_state::WmState,
};

/// Handles session lock/unlock and sleep/resume events.
///
/// On lock or suspend, pauses the WM to prevent it from reacting to
/// windows being hidden by the OS during the locked/suspended state.
///
/// On unlock or resume, unpauses the WM, re-manages any windows that
/// were lost during the lock transition, and performs a full redraw.
pub fn handle_session_change(
  event: &SessionEvent,
  state: &mut WmState,
  config: &mut UserConfig,
) -> anyhow::Result<()> {
  match event {
    SessionEvent::Locked | SessionEvent::Suspending => {
      tracing::info!("Session locked or suspending, pausing WM.");
      state.is_paused = true;
      state.emit_event(WmEvent::PauseChanged { is_paused: true });
      Ok(())
    }
    SessionEvent::Unlocked | SessionEvent::Resumed => {
      tracing::info!(
        "Session unlocked or resumed, unpausing WM and redrawing."
      );
      state.is_paused = false;
      state.emit_event(WmEvent::PauseChanged { is_paused: false });

      // Re-evaluate display settings since monitors may have changed
      // during the lock/sleep (e.g. docking station disconnected).
      handle_display_settings_changed(state, config)?;

      // Re-manage any visible windows that were unmanaged during the
      // lock transition. This handles the race condition where window
      // hidden events are processed before the lock event sets
      // `is_paused`.
      if let Ok(visible_windows) = state.dispatcher.visible_windows() {
        for native_window in visible_windows {
          let is_managed =
            state.window_from_native(&native_window).is_some();

          if !is_managed {
            tracing::info!(
              "Re-managing window after unlock: {:?}",
              native_window,
            );

            let nearest_workspace = state
              .nearest_monitor(&native_window)
              .and_then(|m| m.displayed_workspace());

            if let Some(workspace) = nearest_workspace {
              let _ = manage_window(
                native_window,
                Some(workspace.into()),
                state,
                config,
              );
            }
          }
        }
      }

      // Force a full redraw of the container tree to reposition all
      // managed windows.
      state
        .pending_sync
        .queue_container_to_redraw(state.root_container.clone());

      Ok(())
    }
  }
}
