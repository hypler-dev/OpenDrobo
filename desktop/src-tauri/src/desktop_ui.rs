use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use tauri::{AppHandle, Emitter, Manager, Runtime};

const MAIN_WINDOW_LABEL: &str = "main";
const DESKTOP_ACTION_EVENT: &str = "opendrobo-desktop-action";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DesktopAction {
    Show,
    Status,
    Capacity,
    Volumes,
    Events,
    Statistics,
    Console,
    Refresh,
    Tools,
    Preferences,
    Help,
    WindowHidden,
}

impl DesktopAction {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Show => "show",
            Self::Status => "status",
            Self::Capacity => "capacity",
            Self::Volumes => "volumes",
            Self::Events => "events",
            Self::Statistics => "statistics",
            Self::Console => "console",
            Self::Refresh => "refresh",
            Self::Tools => "tools",
            Self::Preferences => "preferences",
            Self::Help => "help",
            Self::WindowHidden => "window-hidden",
        }
    }

    fn from_menu_id(id: &str) -> Option<Self> {
        match id {
            "show" => Some(Self::Show),
            "status" => Some(Self::Status),
            "capacity" => Some(Self::Capacity),
            "volumes" => Some(Self::Volumes),
            "events" => Some(Self::Events),
            "statistics" => Some(Self::Statistics),
            "console" => Some(Self::Console),
            "refresh" => Some(Self::Refresh),
            "tools" => Some(Self::Tools),
            "preferences" => Some(Self::Preferences),
            "help" => Some(Self::Help),
            _ => None,
        }
    }
}

#[derive(Default)]
struct PendingActions {
    window_hidden: bool,
    latest_navigation: Option<DesktopAction>,
    refresh_requested: bool,
}

#[derive(Default)]
pub struct DesktopActionQueue(Mutex<PendingActions>);

impl DesktopActionQueue {
    fn push(&self, action: DesktopAction) {
        let mut pending = self.0.lock().expect("desktop action queue lock poisoned");
        if action == DesktopAction::WindowHidden {
            // A hidden window must clear frontend control state before any later navigation.
            pending.window_hidden = true;
            pending.latest_navigation = None;
            pending.refresh_requested = false;
        } else if action == DesktopAction::Refresh {
            pending.refresh_requested = true;
        } else if action == DesktopAction::Show {
            // Revealing the window must not erase a queued destination.
            if pending.latest_navigation.is_none() {
                pending.latest_navigation = Some(action);
            }
        } else {
            pending.latest_navigation = Some(action);
        }
    }

    fn take(&self) -> Option<DesktopAction> {
        let mut pending = self.0.lock().expect("desktop action queue lock poisoned");
        if pending.window_hidden {
            pending.window_hidden = false;
            Some(DesktopAction::WindowHidden)
        } else if let Some(action) = pending.latest_navigation.take() {
            Some(action)
        } else if pending.refresh_requested {
            pending.refresh_requested = false;
            Some(DesktopAction::Refresh)
        } else {
            None
        }
    }
}

#[derive(Default)]
pub struct TrayInstalled(AtomicBool);

impl TrayInstalled {
    pub fn mark_installed(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_installed(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

fn publish_action<R: Runtime>(app: &AppHandle<R>, action: DesktopAction) {
    app.state::<DesktopActionQueue>().push(action);
    if let Err(error) = app.emit_to(MAIN_WINDOW_LABEL, DESKTOP_ACTION_EVENT, ()) {
        eprintln!("OpenDrobo desktop action event failed: {error}");
    }
}

pub fn queue_window_hidden<R: Runtime>(app: &AppHandle<R>) {
    publish_action(app, DesktopAction::WindowHidden);
}

fn is_main_window(label: &str) -> bool {
    label == MAIN_WINDOW_LABEL
}

fn reveal_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        if let Err(error) = window
            .show()
            .and_then(|_| window.unminimize())
            .and_then(|_| window.set_focus())
        {
            eprintln!("OpenDrobo could not reveal its main window: {error}");
        }
    }
}

#[tauri::command]
pub fn take_desktop_action(
    window: tauri::WebviewWindow,
    queue: tauri::State<'_, DesktopActionQueue>,
) -> Result<Option<String>, String> {
    if !is_main_window(window.label()) {
        return Err("Desktop actions are available only to the main window.".to_string());
    }
    Ok(queue.take().map(|action| action.as_str().to_string()))
}

pub fn install_tray<R: Runtime, M: Manager<R>>(manager: &M) -> tauri::Result<()> {
    use tauri::{
        menu::{Menu, MenuItem, PredefinedMenuItem},
        tray::TrayIconBuilder,
    };

    let dashboard = MenuItem::with_id(manager, "show", "Open Dashboard", true, None::<&str>)?;
    let status = MenuItem::with_id(manager, "status", "Status", true, None::<&str>)?;
    let capacity = MenuItem::with_id(manager, "capacity", "Capacity", true, None::<&str>)?;
    let volumes = MenuItem::with_id(manager, "volumes", "Volumes", true, None::<&str>)?;
    let events = MenuItem::with_id(manager, "events", "Events", true, None::<&str>)?;
    let statistics = MenuItem::with_id(manager, "statistics", "Statistics", true, None::<&str>)?;
    let console = MenuItem::with_id(manager, "console", "Console", true, None::<&str>)?;
    let refresh = MenuItem::with_id(manager, "refresh", "Refresh", true, None::<&str>)?;
    let tools = MenuItem::with_id(manager, "tools", "Device Tools…", true, None::<&str>)?;
    let preferences = MenuItem::with_id(manager, "preferences", "Preferences", true, None::<&str>)?;
    let help = MenuItem::with_id(manager, "help", "Help", true, None::<&str>)?;
    let hide = MenuItem::with_id(manager, "hide", "Hide", true, None::<&str>)?;
    let quit = MenuItem::with_id(manager, "quit", "Quit", true, None::<&str>)?;
    let section = PredefinedMenuItem::separator(manager)?;
    let footer = PredefinedMenuItem::separator(manager)?;
    let menu = Menu::with_items(
        manager,
        &[
            &dashboard,
            &status,
            &capacity,
            &volumes,
            &events,
            &statistics,
            &console,
            &refresh,
            &section,
            &tools,
            &preferences,
            &help,
            &footer,
            &hide,
            &quit,
        ],
    )?;

    let mut tray = TrayIconBuilder::with_id("opendrobo")
        .menu(&menu)
        .tooltip("OpenDrobo")
        .show_menu_on_left_click(true);
    #[cfg(target_os = "macos")]
    {
        tray = tray.icon(macos_template_icon()).icon_as_template(true);
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Some(icon) = manager.default_window_icon().cloned() {
            tray = tray.icon(icon);
        }
    }

    tray.on_menu_event(|app, event| match event.id().as_ref() {
        "quit" => app.exit(0),
        "hide" => {
            queue_window_hidden(app);
            if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                if let Err(error) = window.hide() {
                    eprintln!("OpenDrobo could not hide its main window: {error}");
                }
            }
        }
        id => {
            if let Some(action) = DesktopAction::from_menu_id(id) {
                reveal_main_window(app);
                publish_action(app, action);
            }
        }
    })
    .build(manager)?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn macos_template_icon() -> tauri::image::Image<'static> {
    const SIZE: usize = 44;
    let mut rgba = vec![0; SIZE * SIZE * 4];
    let mut pixel = |x: usize, y: usize, alpha: u8| {
        let offset = (y * SIZE + x) * 4;
        rgba[offset..offset + 4].copy_from_slice(&[0, 0, 0, alpha]);
    };

    for bay in 0..5 {
        let top = 7 + bay * 6;
        for y in top..top + 5 {
            for x in 10..34 {
                let border = y == top || y == top + 4 || x == 10 || x == 33;
                let handle = (13..16).contains(&x) && y == top + 2;
                let led = x == 30 && y == top + 2;
                if border || handle || led {
                    pixel(x, y, 255);
                }
            }
        }
    }
    for y in 4..40 {
        for x in 8..36 {
            if x < 10 || x > 33 || y < 7 || y > 35 {
                pixel(x, y, 255);
            }
        }
    }
    tauri::image::Image::new_owned(rgba, SIZE as u32, SIZE as u32)
}

#[cfg(test)]
mod tests {
    use super::{DesktopAction, DesktopActionQueue};

    #[test]
    fn routes_only_declared_navigation_actions() {
        for (id, action) in [
            ("show", DesktopAction::Show),
            ("status", DesktopAction::Status),
            ("capacity", DesktopAction::Capacity),
            ("volumes", DesktopAction::Volumes),
            ("events", DesktopAction::Events),
            ("statistics", DesktopAction::Statistics),
            ("console", DesktopAction::Console),
            ("refresh", DesktopAction::Refresh),
            ("tools", DesktopAction::Tools),
            ("preferences", DesktopAction::Preferences),
            ("help", DesktopAction::Help),
        ] {
            assert_eq!(DesktopAction::from_menu_id(id), Some(action));
        }
        assert_eq!(DesktopAction::from_menu_id("execute-device-control"), None);
        assert_eq!(DesktopAction::from_menu_id("quit"), None);
    }

    #[test]
    fn queue_coalesces_navigation_to_latest_action() {
        let queue = DesktopActionQueue::default();
        queue.push(DesktopAction::Status);
        queue.push(DesktopAction::Capacity);
        assert_eq!(queue.take(), Some(DesktopAction::Capacity));
        assert_eq!(queue.take(), None);
    }

    #[test]
    fn hidden_cancellation_is_sticky_before_later_navigation() {
        let queue = DesktopActionQueue::default();
        queue.push(DesktopAction::Tools);
        queue.push(DesktopAction::WindowHidden);
        queue.push(DesktopAction::Show);
        assert_eq!(queue.take(), Some(DesktopAction::WindowHidden));
        assert_eq!(queue.take(), Some(DesktopAction::Show));
        assert_eq!(queue.take(), None);
    }

    #[test]
    fn navigation_and_refresh_survive_show_in_either_click_order() {
        for actions in [
            [
                DesktopAction::Status,
                DesktopAction::Refresh,
                DesktopAction::Show,
            ],
            [
                DesktopAction::Refresh,
                DesktopAction::Status,
                DesktopAction::Show,
            ],
        ] {
            let queue = DesktopActionQueue::default();
            for action in actions {
                queue.push(action);
            }
            assert_eq!(queue.take(), Some(DesktopAction::Status));
            assert_eq!(queue.take(), Some(DesktopAction::Refresh));
            assert_eq!(queue.take(), None);
        }
    }

    #[test]
    fn only_main_window_can_drain_desktop_actions() {
        assert!(super::is_main_window("main"));
        assert!(!super::is_main_window("settings"));
        assert!(!super::is_main_window(""));
    }
}
