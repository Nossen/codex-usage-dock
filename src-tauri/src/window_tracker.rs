use active_win_pos_rs::{ActiveWindow, WindowPosition};
use std::{
    env,
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{AppHandle, Manager};
#[cfg(not(target_os = "windows"))]
use tauri::{LogicalPosition, LogicalSize, Position, Size};
#[cfg(target_os = "windows")]
use tauri::{PhysicalPosition, PhysicalSize, Position, Size};
use tokio::time::sleep;

const POLL_INTERVAL: Duration = Duration::from_millis(400);
const PANEL_INSET: f64 = 16.0;
const PANEL_WIDTH: f64 = 304.0;
const PANEL_HEIGHT: f64 = 326.0;
const COLLAPSED_SIZE: f64 = 58.0;
const MOVE_THRESHOLD: f64 = 2.0;

#[derive(Debug, Clone, Copy)]
struct WindowBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl From<WindowPosition> for WindowBounds {
    fn from(value: WindowPosition) -> Self {
        Self {
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ScreenGeometry {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    #[cfg(target_os = "windows")]
    scale: f64,
}

#[derive(Debug, Default)]
struct PanelLayout {
    collapsed: bool,
    expanded_offset: Option<(f64, f64)>,
    last_codex_window: Option<WindowBounds>,
    layout_dirty: bool,
}

#[derive(Clone, Default)]
pub struct SharedPanelLayout(Arc<Mutex<PanelLayout>>);

impl SharedPanelLayout {
    pub fn set_collapsed(&self, collapsed: bool) {
        let mut layout = self.0.lock().expect("panel layout lock poisoned");
        layout.collapsed = collapsed;
        layout.layout_dirty = true;
    }

    fn snapshot(&self) -> PanelLayout {
        let layout = self.0.lock().expect("panel layout lock poisoned");
        PanelLayout {
            collapsed: layout.collapsed,
            expanded_offset: layout.expanded_offset,
            last_codex_window: layout.last_codex_window,
            layout_dirty: layout.layout_dirty,
        }
    }

    fn remember_codex_window(&self, bounds: WindowBounds) {
        self.0
            .lock()
            .expect("panel layout lock poisoned")
            .last_codex_window = Some(bounds);
    }

    fn remember_expanded_offset(&self, offset: (f64, f64)) {
        self.0
            .lock()
            .expect("panel layout lock poisoned")
            .expanded_offset = Some(offset);
    }

    fn mark_clean(&self, collapsed: bool) {
        let mut layout = self.0.lock().expect("panel layout lock poisoned");
        if layout.collapsed == collapsed {
            layout.layout_dirty = false;
        }
    }
}

pub fn spawn(app: AppHandle, shared_layout: SharedPanelLayout) {
    tauri::async_runtime::spawn(async move {
        let Some(panel) = app.get_webview_window("main") else {
            return;
        };

        if env::var_os("CODEX_USAGE_DOCK_ALWAYS_VISIBLE").is_some() {
            let _ = panel.set_always_on_top(true);
            let _ = panel.show();
            return;
        }

        loop {
            match active_win_pos_rs::get_active_window() {
                Ok(active) if is_own_window(&active) => {
                    let layout = shared_layout.snapshot();
                    if let Some(bounds) = layout.last_codex_window {
                        if layout.layout_dirty {
                            apply_layout(&panel, &bounds, layout.collapsed, layout.expanded_offset);
                            shared_layout.mark_clean(layout.collapsed);
                        } else if !layout.collapsed {
                            remember_manual_position(
                                &panel,
                                &shared_layout,
                                &bounds,
                                layout.expanded_offset,
                            );
                        }
                        let _ = panel.set_always_on_top(true);
                        let _ = panel.show();
                    }
                }
                Ok(active) if is_codex_window(&active) => {
                    let bounds = WindowBounds::from(active.position);
                    shared_layout.remember_codex_window(bounds);
                    let layout = shared_layout.snapshot();
                    apply_layout(&panel, &bounds, layout.collapsed, layout.expanded_offset);
                    shared_layout.mark_clean(layout.collapsed);
                    let _ = panel.set_always_on_top(true);
                    let _ = panel.show();
                }
                _ => {
                    let _ = panel.hide();
                    let _ = panel.set_always_on_top(false);
                }
            }

            sleep(POLL_INTERVAL).await;
        }
    });
}

fn apply_layout(
    panel: &tauri::WebviewWindow,
    bounds: &WindowBounds,
    collapsed: bool,
    expanded_offset: Option<(f64, f64)>,
) {
    let logical_size = if collapsed {
        (COLLAPSED_SIZE, COLLAPSED_SIZE)
    } else {
        (PANEL_WIDTH, PANEL_HEIGHT)
    };
    let screen = screen_for_bounds(panel, bounds);
    let (panel_width, panel_height) = panel_dimensions(logical_size, screen);
    let (x, y) = if collapsed {
        bottom_right_position(bounds, panel_width, panel_height, screen)
    } else {
        expanded_position(bounds, panel_width, panel_height, screen, expanded_offset)
    };

    let _ = set_panel_size(panel, logical_size, screen);
    let _ = set_panel_position(panel, x, y);
}

fn remember_manual_position(
    panel: &tauri::WebviewWindow,
    shared_layout: &SharedPanelLayout,
    bounds: &WindowBounds,
    expanded_offset: Option<(f64, f64)>,
) {
    let Some((x, y)) = current_panel_position(panel) else {
        return;
    };
    let screen = screen_for_bounds(panel, bounds);
    let (panel_width, panel_height) = panel_dimensions((PANEL_WIDTH, PANEL_HEIGHT), screen);
    let (expected_x, expected_y) =
        expanded_position(bounds, panel_width, panel_height, screen, expanded_offset);

    if (x - expected_x).abs() > MOVE_THRESHOLD || (y - expected_y).abs() > MOVE_THRESHOLD {
        shared_layout.remember_expanded_offset((x - bounds.x, y - bounds.y));
    }
}

fn is_codex_window(window: &ActiveWindow) -> bool {
    let app_name = window.app_name.trim().to_ascii_lowercase();
    let process_name = window
        .process_path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    matches!(app_name.as_str(), "codex" | "chatgpt")
        || matches!(process_name.as_str(), "codex" | "chatgpt")
}

fn is_own_window(window: &ActiveWindow) -> bool {
    let app_name = window.app_name.trim().to_ascii_lowercase();
    let process_name = window
        .process_path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    app_name == "codex usage dock"
        || process_name == "codex-usage-dock"
        || process_name == "codex_usage_dock"
}

fn expanded_position(
    bounds: &WindowBounds,
    panel_width: f64,
    panel_height: f64,
    screen: Option<ScreenGeometry>,
    offset: Option<(f64, f64)>,
) -> (f64, f64) {
    match offset {
        Some((offset_x, offset_y)) => clamp_to_screen(
            bounds.x + offset_x,
            bounds.y + offset_y,
            panel_width,
            panel_height,
            screen,
        ),
        None => bottom_right_position(bounds, panel_width, panel_height, screen),
    }
}

fn bottom_right_position(
    bounds: &WindowBounds,
    panel_width: f64,
    panel_height: f64,
    screen: Option<ScreenGeometry>,
) -> (f64, f64) {
    clamp_to_screen(
        bounds.x + bounds.width - panel_width - PANEL_INSET,
        bounds.y + bounds.height - panel_height - PANEL_INSET,
        panel_width,
        panel_height,
        screen,
    )
}

fn clamp_to_screen(
    x: f64,
    y: f64,
    panel_width: f64,
    panel_height: f64,
    screen: Option<ScreenGeometry>,
) -> (f64, f64) {
    let Some(screen) = screen else {
        return (x, y);
    };
    let min_x = screen.x + PANEL_INSET;
    let min_y = screen.y + PANEL_INSET;
    let max_x = (screen.x + screen.width - panel_width - PANEL_INSET).max(min_x);
    let max_y = (screen.y + screen.height - panel_height - PANEL_INSET).max(min_y);

    (x.clamp(min_x, max_x), y.clamp(min_y, max_y))
}

fn screen_for_bounds(
    panel: &tauri::WebviewWindow,
    bounds: &WindowBounds,
) -> Option<ScreenGeometry> {
    let center_x = bounds.x + bounds.width / 2.0;
    let center_y = bounds.y + bounds.height / 2.0;
    let monitors = panel.available_monitors().ok()?;

    monitors.into_iter().find_map(|monitor| {
        let scale = monitor.scale_factor();
        let position = monitor.position();
        let size = monitor.size();

        #[cfg(target_os = "windows")]
        let geometry = ScreenGeometry {
            x: f64::from(position.x),
            y: f64::from(position.y),
            width: f64::from(size.width),
            height: f64::from(size.height),
            scale,
        };

        #[cfg(not(target_os = "windows"))]
        let geometry = ScreenGeometry {
            x: f64::from(position.x) / scale,
            y: f64::from(position.y) / scale,
            width: f64::from(size.width) / scale,
            height: f64::from(size.height) / scale,
        };

        let contains_center = center_x >= geometry.x
            && center_x < geometry.x + geometry.width
            && center_y >= geometry.y
            && center_y < geometry.y + geometry.height;
        contains_center.then_some(geometry)
    })
}

#[cfg(target_os = "windows")]
fn panel_dimensions(logical_size: (f64, f64), screen: Option<ScreenGeometry>) -> (f64, f64) {
    let scale = screen.map(|value| value.scale).unwrap_or(1.0);
    (logical_size.0 * scale, logical_size.1 * scale)
}

#[cfg(not(target_os = "windows"))]
fn panel_dimensions(logical_size: (f64, f64), _screen: Option<ScreenGeometry>) -> (f64, f64) {
    logical_size
}

#[cfg(target_os = "windows")]
fn current_panel_position(panel: &tauri::WebviewWindow) -> Option<(f64, f64)> {
    let position = panel.outer_position().ok()?;
    Some((f64::from(position.x), f64::from(position.y)))
}

#[cfg(not(target_os = "windows"))]
fn current_panel_position(panel: &tauri::WebviewWindow) -> Option<(f64, f64)> {
    let position = panel.outer_position().ok()?;
    let scale = panel.scale_factor().ok()?;
    Some((f64::from(position.x) / scale, f64::from(position.y) / scale))
}

#[cfg(target_os = "macos")]
fn set_panel_size(
    panel: &tauri::WebviewWindow,
    logical_size: (f64, f64),
    _screen: Option<ScreenGeometry>,
) -> tauri::Result<()> {
    panel.set_size(Size::Logical(LogicalSize::new(
        logical_size.0,
        logical_size.1,
    )))
}

#[cfg(target_os = "windows")]
fn set_panel_size(
    panel: &tauri::WebviewWindow,
    logical_size: (f64, f64),
    screen: Option<ScreenGeometry>,
) -> tauri::Result<()> {
    let scale = screen.map(|value| value.scale).unwrap_or(1.0);
    panel.set_size(Size::Physical(PhysicalSize::new(
        (logical_size.0 * scale).round() as u32,
        (logical_size.1 * scale).round() as u32,
    )))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn set_panel_size(
    panel: &tauri::WebviewWindow,
    logical_size: (f64, f64),
    _screen: Option<ScreenGeometry>,
) -> tauri::Result<()> {
    panel.set_size(Size::Logical(LogicalSize::new(
        logical_size.0,
        logical_size.1,
    )))
}

#[cfg(target_os = "macos")]
fn set_panel_position(panel: &tauri::WebviewWindow, x: f64, y: f64) -> tauri::Result<()> {
    panel.set_position(Position::Logical(LogicalPosition::new(x, y)))
}

#[cfg(target_os = "windows")]
fn set_panel_position(panel: &tauri::WebviewWindow, x: f64, y: f64) -> tauri::Result<()> {
    panel.set_position(Position::Physical(PhysicalPosition::new(
        x.round() as i32,
        y.round() as i32,
    )))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn set_panel_position(panel: &tauri::WebviewWindow, x: f64, y: f64) -> tauri::Result<()> {
    panel.set_position(Position::Logical(LogicalPosition::new(x, y)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn active(app_name: &str, process: &str) -> ActiveWindow {
        ActiveWindow {
            app_name: app_name.into(),
            process_path: PathBuf::from(process),
            ..ActiveWindow::default()
        }
    }

    #[test]
    fn recognizes_current_and_future_codex_process_names() {
        assert!(is_codex_window(&active("ChatGPT", "/Applications/ChatGPT")));
        assert!(is_codex_window(&active(
            "Codex",
            "C:\\Program Files\\Codex.exe"
        )));
        assert!(!is_codex_window(&active("Safari", "/Applications/Safari")));
    }

    #[test]
    fn does_not_misclassify_the_dock_as_codex() {
        let dock = active("Codex Usage Dock", "/Applications/Codex Usage Dock");
        assert!(is_own_window(&dock));
        assert!(!is_codex_window(&dock));
    }

    #[test]
    fn defaults_to_the_bottom_right_inside_codex() {
        let bounds = WindowBounds {
            x: 100.0,
            y: 50.0,
            width: 800.0,
            height: 600.0,
        };
        let screen = ScreenGeometry {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0,
            #[cfg(target_os = "windows")]
            scale: 1.0,
        };
        assert_eq!(
            expanded_position(&bounds, PANEL_WIDTH, PANEL_HEIGHT, Some(screen), None),
            (580.0, 308.0)
        );
    }

    #[test]
    fn collapsed_icon_sits_in_the_bottom_right() {
        let bounds = WindowBounds {
            x: 100.0,
            y: 50.0,
            width: 800.0,
            height: 600.0,
        };
        let screen = ScreenGeometry {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0,
            #[cfg(target_os = "windows")]
            scale: 1.0,
        };
        assert_eq!(
            bottom_right_position(&bounds, COLLAPSED_SIZE, COLLAPSED_SIZE, Some(screen)),
            (826.0, 576.0)
        );
    }

    #[test]
    fn keeps_a_dragged_position_relative_to_codex_and_on_screen() {
        let bounds = WindowBounds {
            x: 100.0,
            y: 50.0,
            width: 800.0,
            height: 600.0,
        };
        let screen = ScreenGeometry {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0,
            #[cfg(target_os = "windows")]
            scale: 1.0,
        };
        assert_eq!(
            expanded_position(
                &bounds,
                PANEL_WIDTH,
                PANEL_HEIGHT,
                Some(screen),
                Some((40.0, 80.0))
            ),
            (140.0, 130.0)
        );
        assert_eq!(
            expanded_position(
                &bounds,
                PANEL_WIDTH,
                PANEL_HEIGHT,
                Some(screen),
                Some((2_000.0, 2_000.0))
            ),
            (1120.0, 558.0)
        );
    }
}
