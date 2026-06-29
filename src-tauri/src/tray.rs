//! 系统托盘：菜单（显示主窗口 / 退出）+ 左键点击唤起窗口。
//!
//! 托盘图标按平台规范区分：
//! - macOS 菜单栏走 template image（纯黑 + 透明），系统按明暗主题自动反色
//! - Windows 通知区走单色白 PNG
//! 图标通过 `include_bytes!` 编译进二进制，运行时无路径依赖。

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

pub fn init_tray(app: &AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "tray_show", "显示主窗口", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "tray_quit", "退出月神代理", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    #[cfg(target_os = "macos")]
    const TRAY_ICON_BYTES: &[u8] = include_bytes!("assets/tray_macos_44.png");
    #[cfg(target_os = "windows")]
    const TRAY_ICON_BYTES: &[u8] = include_bytes!("assets/tray_windows_64.png");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    const TRAY_ICON_BYTES: &[u8] = include_bytes!("assets/tray_windows_64.png");

    let tray_icon = tauri::image::Image::from_bytes(TRAY_ICON_BYTES)?;

    TrayIconBuilder::with_id("main-tray")
        .icon(tray_icon)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("月神代理")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "tray_show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
            }
            "tray_quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
            }
        })
        .build(app)?;
    Ok(())
}
