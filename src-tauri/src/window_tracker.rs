use active_win_pos_rs::{ActiveWindow, WindowPosition};
use std::{env, time::Duration};
use tauri::{AppHandle, Manager};
#[cfg(not(target_os = "windows"))]
use tauri::{LogicalPosition, Position};
#[cfg(target_os = "windows")]
use tauri::{PhysicalPosition, Position};
use tokio::time::sleep;

const POLL_INTERVAL: Duration = Duration::from_millis(400);
const PANEL_GAP: f64 = 12.0;
const PANEL_TOP_OFFSET: f64 = 44.0;
const PANEL_INSET: f64 = 16.0;
const PANEL_WIDTH: f64 = 304.0;
const PANEL_HEIGHT: f64 = 326.0;

#[derive(Debug, Clone, Copy)]
struct ScreenGeometry {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    panel_width: f64,
    panel_height: f64,
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let Some(panel) = app.get_webview_window("main") else {
            return;
        };

        if env::var_os("CODEX_USAGE_DOCK_ALWAYS_VISIBLE").is_some() {
            let _ = panel.set_always_on_top(true);
            let _ = panel.show();
            return;
        }

        let mut last_codex_window: Option<WindowPosition> = None;

        loop {
            match active_win_pos_rs::get_active_window() {
                Ok(active) if is_own_window(&active) => {
                    if last_codex_window.is_some() {
                        let _ = panel.set_always_on_top(true);
                        let _ = panel.show();
                    }
                }
                Ok(active) if is_codex_window(&active) => {
                    let bounds = active.position;
                    let screen = screen_for_bounds(&panel, &bounds);
                    let (x, y) = dock_position(&bounds, screen);
                    let _ = set_panel_position(&panel, x, y);
                    let _ = panel.set_always_on_top(true);
                    let _ = panel.show();
                    last_codex_window = Some(bounds);
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

fn dock_position(bounds: &WindowPosition, screen: Option<ScreenGeometry>) -> (f64, f64) {
    let outside_x = bounds.x + bounds.width + PANEL_GAP;
    let mut y = bounds.y + PANEL_TOP_OFFSET;

    let Some(screen) = screen else {
        return (outside_x, y);
    };

    let screen_right = screen.x + screen.width;
    let screen_bottom = screen.y + screen.height;
    let x = if outside_x + screen.panel_width <= screen_right {
        outside_x
    } else {
        (bounds.x + bounds.width - screen.panel_width - PANEL_INSET).max(screen.x + PANEL_INSET)
    };

    y = y
        .max(screen.y + PANEL_INSET)
        .min(screen_bottom - screen.panel_height - PANEL_INSET);

    (x, y)
}

fn screen_for_bounds(
    panel: &tauri::WebviewWindow,
    bounds: &WindowPosition,
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
            panel_width: PANEL_WIDTH * scale,
            panel_height: PANEL_HEIGHT * scale,
        };

        #[cfg(not(target_os = "windows"))]
        let geometry = ScreenGeometry {
            x: f64::from(position.x) / scale,
            y: f64::from(position.y) / scale,
            width: f64::from(size.width) / scale,
            height: f64::from(size.height) / scale,
            panel_width: PANEL_WIDTH,
            panel_height: PANEL_HEIGHT,
        };

        let contains_center = center_x >= geometry.x
            && center_x < geometry.x + geometry.width
            && center_y >= geometry.y
            && center_y < geometry.y + geometry.height;
        contains_center.then_some(geometry)
    })
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
    fn docks_to_the_right_with_a_small_top_offset() {
        let bounds = WindowPosition::new(100.0, 50.0, 800.0, 600.0);
        let screen = ScreenGeometry {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0,
            panel_width: PANEL_WIDTH,
            panel_height: PANEL_HEIGHT,
        };
        assert_eq!(dock_position(&bounds, Some(screen)), (912.0, 94.0));
    }

    #[test]
    fn falls_back_inside_a_maximized_codex_window() {
        let bounds = WindowPosition::new(0.0, 0.0, 1440.0, 900.0);
        let screen = ScreenGeometry {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0,
            panel_width: PANEL_WIDTH,
            panel_height: PANEL_HEIGHT,
        };
        assert_eq!(dock_position(&bounds, Some(screen)), (1120.0, 44.0));
    }
}
