use std::path::PathBuf;

use crate::model::{ConfigError, ConfigModel, Record};
use crate::scheduler::SignalId;

/// Persisted window geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
    pub fullscreen: bool,
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 1280,
            height: 720,
            maximized: false,
            fullscreen: false,
        }
    }
}

impl Record for WindowGeometry {
    fn from_kdl(node: &kdl::KdlNode) -> Result<Self, ConfigError> {
        let x = node
            .get("x")
            .and_then(|e| e.as_integer())
            .map(|v| v as i32)
            .ok_or_else(|| ConfigError::MissingField("x".into()))?;
        let y = node
            .get("y")
            .and_then(|e| e.as_integer())
            .map(|v| v as i32)
            .ok_or_else(|| ConfigError::MissingField("y".into()))?;
        let width = node
            .get("width")
            .and_then(|e| e.as_integer())
            .map(|v| v as u32)
            .ok_or_else(|| ConfigError::MissingField("width".into()))?;
        let height = node
            .get("height")
            .and_then(|e| e.as_integer())
            .map(|v| v as u32)
            .ok_or_else(|| ConfigError::MissingField("height".into()))?;
        let maximized = node
            .get("maximized")
            .and_then(|e| e.as_bool())
            .unwrap_or(false);
        let fullscreen = node
            .get("fullscreen")
            .and_then(|e| e.as_bool())
            .unwrap_or(false);

        Ok(Self {
            x,
            y,
            width,
            height,
            maximized,
            fullscreen,
        })
    }

    fn to_kdl(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("window-geometry");
        node.push(kdl::KdlEntry::new_prop("x", self.x as i128));
        node.push(kdl::KdlEntry::new_prop("y", self.y as i128));
        node.push(kdl::KdlEntry::new_prop("width", self.width as i128));
        node.push(kdl::KdlEntry::new_prop("height", self.height as i128));
        node.push(kdl::KdlEntry::new_prop("maximized", self.maximized));
        node.push(kdl::KdlEntry::new_prop("fullscreen", self.fullscreen));
        node
    }

    fn set_to_default(&mut self) {
        *self = Self::default();
    }

    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// Saves and restores window geometry via a ConfigModel.
pub struct WindowStateSaver {
    model: ConfigModel<WindowGeometry>,
    /// Cached normal-mode geometry, preserved when maximized/fullscreen.
    /// Matches C++ OwnNormalX/Y/W/H.
    normal_x: i32,
    normal_y: i32,
    normal_w: u32,
    normal_h: u32,
}

impl WindowStateSaver {
    pub fn new(path: PathBuf, signal_id: SignalId) -> Self {
        let defaults = WindowGeometry::default();
        Self {
            normal_x: defaults.x,
            normal_y: defaults.y,
            normal_w: defaults.width,
            normal_h: defaults.height,
            model: ConfigModel::new(defaults, path, signal_id),
        }
    }

    /// Save the current window position/size.
    ///
    /// When maximized or fullscreen, the last normal-mode geometry is
    /// preserved (matching C++ emWindowStateSaver::Save behavior).
    pub fn save_from(&mut self, window: &super::zui_window::ZuiWindow) {
        use super::zui_window::WindowFlags;

        let pos = window.winit_window.outer_position().unwrap_or_default();
        let size = window.winit_window.inner_size();
        let maximized = window.flags.contains(WindowFlags::MAXIMIZED);
        let fullscreen = window.flags.contains(WindowFlags::FULLSCREEN);

        // Only update normal geometry when NOT maximized/fullscreen.
        if !maximized && !fullscreen {
            self.normal_x = pos.x;
            self.normal_y = pos.y;
            self.normal_w = size.width;
            self.normal_h = size.height;
        }

        self.model.set(WindowGeometry {
            x: self.normal_x,
            y: self.normal_y,
            width: self.normal_w,
            height: self.normal_h,
            maximized,
            fullscreen,
        });
    }

    /// Get the stored geometry for restoring.
    pub fn geometry(&self) -> &WindowGeometry {
        self.model.get()
    }

    pub fn cycle(&mut self, window: &super::zui_window::ZuiWindow, focused: bool) {
        use super::zui_window::WindowFlags;

        let pos = window.winit_window.outer_position().unwrap_or_default();
        let size = window.winit_window.inner_size();
        let maximized = window.flags.contains(WindowFlags::MAXIMIZED);
        let fullscreen = window.flags.contains(WindowFlags::FULLSCREEN);

        let current = WindowGeometry {
            x: if !maximized && !fullscreen {
                pos.x
            } else {
                self.normal_x
            },
            y: if !maximized && !fullscreen {
                pos.y
            } else {
                self.normal_y
            },
            width: if !maximized && !fullscreen {
                size.width
            } else {
                self.normal_w
            },
            height: if !maximized && !fullscreen {
                size.height
            } else {
                self.normal_h
            },
            maximized,
            fullscreen,
        };

        if focused && current != *self.model.get() {
            self.save_from(window);
        }
    }

    pub fn model(&self) -> &ConfigModel<WindowGeometry> {
        &self.model
    }
}
