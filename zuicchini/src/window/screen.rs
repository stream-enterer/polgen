/// Information about a physical monitor.
#[derive(Clone, Debug)]
pub struct MonitorInfo {
    pub name: Option<String>,
    pub position: (i32, i32),
    pub size: (u32, u32),
    pub scale_factor: f64,
    pub primary: bool,
}

/// Tracks available monitors and virtual desktop bounds.
pub struct Screen {
    monitors: Vec<MonitorInfo>,
    /// Virtual desktop bounding box (x, y, w, h).
    pub virtual_bounds: (i32, i32, u32, u32),
}

impl Screen {
    /// Populate from winit's available monitors.
    pub fn from_event_loop(event_loop: &winit::event_loop::ActiveEventLoop) -> Self {
        let mut monitors = Vec::new();
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        let mut first = true;
        for handle in event_loop.available_monitors() {
            let pos = handle.position();
            let size = handle.size();
            let scale = handle.scale_factor();
            let name = handle.name();

            let info = MonitorInfo {
                name,
                position: (pos.x, pos.y),
                size: (size.width, size.height),
                scale_factor: scale,
                primary: first,
            };
            first = false;

            min_x = min_x.min(pos.x);
            min_y = min_y.min(pos.y);
            max_x = max_x.max(pos.x + size.width as i32);
            max_y = max_y.max(pos.y + size.height as i32);

            monitors.push(info);
        }

        let virtual_bounds = if monitors.is_empty() {
            (0, 0, 1920, 1080)
        } else {
            (min_x, min_y, (max_x - min_x) as u32, (max_y - min_y) as u32)
        };

        Self {
            monitors,
            virtual_bounds,
        }
    }

    pub fn monitors(&self) -> &[MonitorInfo] {
        &self.monitors
    }

    pub fn primary(&self) -> Option<&MonitorInfo> {
        self.monitors.iter().find(|m| m.primary)
    }

    /// Return the DPI (dots per inch) of the primary monitor.
    ///
    /// Matches C++ emScreen::GetDPI (pure virtual). Uses the primary monitor's
    /// scale_factor to compute logical DPI. Returns 96.0 as the base DPI
    /// multiplied by the scale factor, following the convention that 1.0 scale
    /// = 96 DPI.
    pub fn get_dpi(&self) -> f64 {
        let scale = self.primary().map(|m| m.scale_factor).unwrap_or(1.0);
        96.0 * scale
    }

    /// Whether the mouse pointer can be moved programmatically.
    ///
    /// Matches C++ emScreen::CanMoveMousePointer. Winit does not support
    /// programmatic relative mouse movement on all platforms.
    pub fn can_move_mouse_pointer(&self) -> bool {
        false
    }

    /// Move the mouse pointer by (dx, dy) pixels.
    ///
    /// Matches C++ emScreen::MoveMousePointer. No-op; winit limitation.
    pub fn move_mouse_pointer(&self, _dx: f64, _dy: f64) {
        // Not supported by winit core. See ZuiWindow::move_mouse_pointer.
    }

    /// Emit an acoustic warning beep.
    ///
    /// Matches C++ emScreen::Beep. No-op; winit limitation.
    pub fn beep(&self) {
        // Not supported by winit. See ZuiWindow::beep.
    }

    /// Find the monitor with maximum overlap area with the given rect.
    pub fn monitor_index_of_rect(&self, x: i32, y: i32, w: u32, h: u32) -> Option<usize> {
        let rx1 = x as i64;
        let ry1 = y as i64;
        let rx2 = rx1 + w as i64;
        let ry2 = ry1 + h as i64;

        let mut best_idx = None;
        let mut best_area: i64 = 0;

        for (i, m) in self.monitors.iter().enumerate() {
            let mx1 = m.position.0 as i64;
            let my1 = m.position.1 as i64;
            let mx2 = mx1 + m.size.0 as i64;
            let my2 = my1 + m.size.1 as i64;

            let ox = (rx2.min(mx2) - rx1.max(mx1)).max(0);
            let oy = (ry2.min(my2) - ry1.max(my1)).max(0);
            let area = ox * oy;

            if area > best_area {
                best_area = area;
                best_idx = Some(i);
            }
        }

        best_idx
    }

    pub fn leave_fullscreen_modes(
        &self,
        windows: &mut std::collections::HashMap<
            winit::window::WindowId,
            super::zui_window::ZuiWindow,
        >,
        except: Option<winit::window::WindowId>,
    ) {
        use super::zui_window::WindowFlags;

        for (id, win) in windows.iter_mut() {
            if win.flags.contains(WindowFlags::FULLSCREEN) && Some(*id) != except {
                win.set_window_flags(win.flags & !WindowFlags::FULLSCREEN);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_screen(monitors: Vec<MonitorInfo>) -> Screen {
        Screen {
            monitors,
            virtual_bounds: (0, 0, 3840, 1080),
        }
    }

    #[test]
    fn monitor_index_of_rect_single() {
        let screen = make_screen(vec![MonitorInfo {
            name: None,
            position: (0, 0),
            size: (1920, 1080),
            scale_factor: 1.0,
            primary: true,
        }]);
        assert_eq!(screen.monitor_index_of_rect(100, 100, 200, 200), Some(0));
    }

    #[test]
    fn monitor_index_of_rect_picks_max_overlap() {
        let screen = make_screen(vec![
            MonitorInfo {
                name: None,
                position: (0, 0),
                size: (1920, 1080),
                scale_factor: 1.0,
                primary: true,
            },
            MonitorInfo {
                name: None,
                position: (1920, 0),
                size: (1920, 1080),
                scale_factor: 1.0,
                primary: false,
            },
        ]);
        // Mostly on monitor 1 (right)
        assert_eq!(screen.monitor_index_of_rect(1900, 0, 200, 100), Some(1));
        // Mostly on monitor 0 (left)
        assert_eq!(screen.monitor_index_of_rect(1800, 0, 200, 100), Some(0));
    }

    #[test]
    fn monitor_index_of_rect_no_overlap() {
        let screen = make_screen(vec![MonitorInfo {
            name: None,
            position: (0, 0),
            size: (1920, 1080),
            scale_factor: 1.0,
            primary: true,
        }]);
        assert_eq!(screen.monitor_index_of_rect(2000, 2000, 100, 100), None);
    }

    #[test]
    fn monitor_index_of_rect_empty_monitors() {
        let screen = make_screen(vec![]);
        assert_eq!(screen.monitor_index_of_rect(0, 0, 100, 100), None);
    }
}
