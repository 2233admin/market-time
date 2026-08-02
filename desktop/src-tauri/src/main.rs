#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    Manager,
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

#[cfg(test)]
mod build_config;

const WINDOW_LABEL: &str = "widget";

fn show_widget(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(WINDOW_LABEL) else {
        eprintln!("Mark Time widget window is unavailable");
        return;
    };
    if let Err(error) = window.show() {
        eprintln!("failed to show Mark Time widget: {error}");
        return;
    }
    if let Err(error) = window.set_focus() {
        eprintln!("failed to focus Mark Time widget: {error}");
    }
}

fn tray_icon() -> Image<'static> {
    const SIZE: u32 = 16;
    let mut rgba = vec![0_u8; (SIZE * SIZE * 4) as usize];

    for y in 1..15 {
        for x in 1..15 {
            let index = ((y * SIZE + x) * 4) as usize;
            let on_ring = !(3..=12).contains(&x) || !(3..=12).contains(&y);
            let on_hand = (x == 8 && (4..=8).contains(&y)) || (y == 8 && (8..=12).contains(&x));

            if on_ring || on_hand {
                rgba[index..index + 4].copy_from_slice(&[234, 179, 8, 255]);
            }
        }
    }

    Image::new_owned(rgba, SIZE, SIZE)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "显示", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "隐藏", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &hide, &quit])?;

            TrayIconBuilder::new()
                .icon(tray_icon())
                .tooltip("Mark Time")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_widget(app),
                    "hide" => {
                        if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
                            if let Err(error) = window.hide() {
                                eprintln!("failed to hide Mark Time widget: {error}");
                            }
                        } else {
                            eprintln!("Mark Time widget window is unavailable");
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show_widget(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                match window.hide() {
                    Ok(()) => api.prevent_close(),
                    Err(error) => {
                        eprintln!("failed to hide Mark Time widget on close: {error}");
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to run Mark Time desktop widget");
}
