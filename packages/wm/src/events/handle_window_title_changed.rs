use tracing::info;
use wm_common::{try_warn, WindowRuleEvent};
use wm_platform::NativeWindow;

use crate::{
  commands::window::{manage_window, run_window_rules},
  traits::WindowGetters,
  user_config::UserConfig,
  wm_state::WmState,
};

pub fn handle_window_title_changed(
  native_window: &NativeWindow,
  state: &mut WmState,
  config: &mut UserConfig,
) -> anyhow::Result<()> {
  let found_window = state.window_from_native(native_window);

  if let Some(window) = found_window {
    info!("Window title changed: {window}");

    let title = try_warn!(window.native().title());

    window.update_native_properties(|properties| {
      properties.title = title;
    });

    // Run window rules for title change events.
    run_window_rules(
      window,
      &WindowRuleEvent::TitleChange,
      state,
      config,
    )?;
  } else {
    // Title-change event for a window not tracked by the WM. On macOS, the
    // initial `manage_window` call from `WindowEvent::Shown` can fail
    // silently if AX queries return `cannot_complete` while the window is
    // still initializing. Retry management here.
    info!("Attempting to manage previously-unmanaged window on title change.");
    manage_window(native_window.clone(), None, state, config)?;
  }

  Ok(())
}
