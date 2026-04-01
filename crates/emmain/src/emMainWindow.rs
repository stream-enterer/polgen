// Port of C++ emMainWindow.
//
// DIVERGED: C++ emMainWindow creates an OS window + emMainPanel + detached
// control window + StartupEngine.  Rust creates a single ZuiWindow with
// emMainPanel as the root panel.  CreateControlWindow and DoCustomCheat are
// added (see create_control_window / do_custom_cheat below) but full runtime
// wiring (raise existing window, link to content view) requires Phase 3's
// startup engine integration.  The startup animation remains deferred.

use std::rc::Rc;

use winit::event_loop::ActiveEventLoop;

use emcore::emGUIFramework::App;
use emcore::emWindow::{WindowFlags, ZuiWindow};

use crate::emMainControlPanel::emMainControlPanel;
use crate::emMainPanel::emMainPanel;

/// Configuration for creating an emMainWindow.
pub struct emMainWindowConfig {
    pub geometry: Option<String>, // "WxH+X+Y"
    pub fullscreen: bool,
    pub visit: Option<String>,
    pub control_tallness: f64,
}

impl Default for emMainWindowConfig {
    fn default() -> Self {
        Self {
            geometry: None,
            fullscreen: false,
            visit: None,
            control_tallness: 5.0,
        }
    }
}

/// Create an emMainWindow: inserts the root emMainPanel into the panel tree,
/// allocates signals, and creates the ZuiWindow.
///
/// Called from the setup callback inside the `App` event loop.
pub fn create_main_window(
    app: &mut App,
    event_loop: &ActiveEventLoop,
    config: &emMainWindowConfig,
) {
    // Create root panel in the tree
    let panel = emMainPanel::new(Rc::clone(&app.context), config.control_tallness);
    let root_id = app.tree.create_root("root");
    app.tree.set_behavior(root_id, Box::new(panel));

    // Determine flags
    let mut flags = WindowFlags::AUTO_DELETE;
    if config.fullscreen {
        flags |= WindowFlags::FULLSCREEN;
    }

    let close_signal = app.scheduler.borrow_mut().create_signal();
    let flags_signal = app.scheduler.borrow_mut().create_signal();

    // Create the window
    let window = ZuiWindow::create(
        event_loop,
        app.gpu(),
        root_id,
        flags,
        close_signal,
        flags_signal,
    );
    let window_id = window.winit_window.id();
    app.windows.insert(window_id, window);
}

/// Create a detached control window.
///
/// Port of C++ `emMainWindow::CreateControlWindow` (emMainWindow.cpp:309-327).
/// Creates a second OS window with `WF_AUTO_DELETE`, hosting an
/// `emMainControlPanel`.
///
/// Triggered by the `"ccw"` cheat code in `DoCustomCheat`.
///
/// Note: Full wiring (raise existing window, link to content view) requires
/// Phase 3's startup engine integration. This establishes the API shape.
pub fn create_control_window(
    app: &mut App,
    event_loop: &ActiveEventLoop,
) -> Option<winit::window::WindowId> {
    let ctrl_panel = emMainControlPanel::new(Rc::clone(&app.context));
    let root_id = app.tree.create_root("ctrl_window_root");
    app.tree.set_behavior(root_id, Box::new(ctrl_panel));

    let flags = WindowFlags::AUTO_DELETE;
    let close_signal = app.scheduler.borrow_mut().create_signal();
    let flags_signal = app.scheduler.borrow_mut().create_signal();

    let window = ZuiWindow::create(
        event_loop,
        app.gpu(),
        root_id,
        flags,
        close_signal,
        flags_signal,
    );
    let window_id = window.winit_window.id();
    app.windows.insert(window_id, window);
    Some(window_id)
}

/// Handle a custom cheat code.
///
/// Port of C++ `emMainWindow::DoCustomCheat` (emMainWindow.cpp:266-277).
///
/// Currently recognized cheats:
/// - `"ccw"`: Create a detached control window.
pub fn do_custom_cheat(cheat: &str, app: &mut App, event_loop: &ActiveEventLoop) {
    match cheat {
        "ccw" => {
            create_control_window(app, event_loop);
        }
        _ => {
            log::debug!("Unknown cheat code: {cheat}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = emMainWindowConfig::default();
        assert!(!config.fullscreen);
        assert!(config.visit.is_none());
        assert!(config.geometry.is_none());
        assert!((config.control_tallness - 5.0).abs() < 1e-10);
    }
}
