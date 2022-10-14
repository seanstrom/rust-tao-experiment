// Copyright 2014-2021 The winit contributors
// Copyright 2021-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

// System tray isn't supported on other's platforms.
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn main() {
    println!("This platform doesn't support system_tray.");
}

// Tray feature flag disabled but can be available.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[cfg(not(feature = "tray"))]
fn main() {
    println!("This platform doesn't have the `tray` feature enabled.");
}

// System tray is supported and availabled only if `tray` feature is enabled.
// Platform: Windows, Linux and macOS.
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[cfg(any(feature = "tray", all(target_os = "linux", feature = "ayatana")))]
fn main() {
    #[cfg(target_os = "linux")]
    use tao::platform::linux::SystemTrayBuilderExtLinux;

    #[cfg(target_os = "macos")]
    use tao::platform::macos::{
        ActivationPolicy, EventLoopExtMacOS, SystemTrayBuilderExtMacOS, SystemTrayExtMacOS,
    };

    use std::time::{Duration, Instant};
    use tao::{
        event::{Event, StartCause},
        event_loop::{ControlFlow, EventLoop},
        menu::{ContextMenu as Menu, MenuItemAttributes, MenuType, MenuItem},
        system_tray::SystemTrayBuilder,
        TrayId, window::Icon,
    };

    // Types

    #[derive(PartialEq)]
    enum Status {
        Idle,
        Running,
        Paused,
    }

    #[cfg(not(target_os = "macos"))]
    let event_loop = EventLoop::new();

    #[cfg(target_os = "macos")]
    let mut event_loop = EventLoop::new();

    #[cfg(target_os = "macos")]
    event_loop.set_activation_policy(ActivationPolicy::Accessory);

    let mut current_status: Status = Status::Idle;
    let mut current_time_left: Duration = Duration::new(0, 0);
    let one_second = Duration::new(1, 0);
    let zero_seconds = Duration::new(0, 0);
    let twenty_minutes = Duration::new(20 * 60, 0);

    fn format_number(number: u64) -> String {
        if number < 10 {
            "0".to_string() + &number.to_string()
        } else {
            number.to_string()
        }
    }

    fn format_timer(current_time_left: Duration) -> String {
        let time_left_seconds = current_time_left.as_secs();
        let remaining_minutes = time_left_seconds / 60;
        let remaining_seconds = time_left_seconds - (remaining_minutes * 60);
        format!(
            "{}:{}",
            format_number(remaining_minutes),
            format_number(remaining_seconds)
        )
    }

    fn control_wait_until(timer: Duration) -> ControlFlow {
        ControlFlow::WaitUntil(Instant::now() + timer)
    }

    // Tray Platform

    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/icons/timer.png");
    let main_tray_id = TrayId::new("main-tray");
    let icon = load_icon(std::path::Path::new(path));
    let mut tray_menu = Menu::new();
    let menu_item = tray_menu.add_item(MenuItemAttributes::new("Clear"));
    let quit = tray_menu.add_native_item(MenuItem::Quit).unwrap();

    #[cfg(target_os = "linux")]
    let system_tray = SystemTrayBuilder::new(icon.clone(), Some(tray_menu))
        .with_id(main_tray_id)
        .with_temp_icon_dir(std::path::Path::new("/tmp/tao-examples"))
        .build(&event_loop)
        .unwrap();

    #[cfg(target_os = "windows")]
    let system_tray = SystemTrayBuilder::new(icon.clone(), Some(tray_menu))
        .with_id(main_tray_id)
        .with_tooltip("tao - windowing creation library")
        .build(&event_loop)
        .unwrap();

    #[cfg(target_os = "macos")]
    let system_tray = SystemTrayBuilder::new(icon.clone(), Some(tray_menu))
        .with_id(main_tray_id)
        .with_title(&format_timer(zero_seconds))
        .with_menu_on_left_click(false)
        .with_tooltip("totodoro - timer")
        .build(&event_loop)
        .unwrap();

    let mut system_tray = Some(system_tray);

    // Event Loop

    event_loop.run(move |event, _event_loop, control_flow| {
        match event {
            Event::NewEvents(StartCause::Init) => {
                println!("Init");
                *control_flow = ControlFlow::Wait;
            }
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                if current_status == Status::Running {
                    println!("Tick");
                    current_time_left = current_time_left - one_second;
                    if let Some(tray) = system_tray.as_mut() {
                        tray.set_title(&format_timer(current_time_left));
                        *control_flow = control_wait_until(one_second);
                    }
                }
            }
            Event::MenuEvent {
                menu_id,
                origin: MenuType::ContextMenu,
                ..
            } => {
                if menu_id == quit.clone().id() {
                    // drop the system tray before exiting to remove the icon from system tray on Windows
                    system_tray.take();
                    *control_flow = ControlFlow::Exit;
                } else if menu_id == menu_item.clone().id() {
                    #[cfg(target_os = "macos")]
                    {
                        if let Some(tray) = system_tray.as_mut() {
                            current_status = Status::Idle;
                            current_time_left = zero_seconds;
                            tray.set_title(&format_timer(current_time_left));
                            *control_flow = ControlFlow::Wait;
                        }
                    }
                }
            }
            Event::TrayEvent { id, event, .. } => {
                if id == main_tray_id {
                    match event {
                        tao::event::TrayEvent::LeftClick => {
                            if let Some(tray) = system_tray.as_mut() {
                                match current_status {
                                    Status::Idle => {
                                        current_status = Status::Running;
                                        current_time_left = twenty_minutes;
                                        tray.set_title(&format_timer(current_time_left));
                                        *control_flow = control_wait_until(one_second);
                                    }
                                    Status::Running => {
                                        current_status = Status::Paused;
                                        *control_flow = ControlFlow::Wait;
                                        tray.set_title(&format_timer(current_time_left));
                                    }
                                    Status::Paused => {
                                        current_status = Status::Running;
                                        *control_flow = control_wait_until(one_second);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => (),
        }
    });
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
#[cfg(any(feature = "tray", all(target_os = "linux", feature = "ayatana")))]
fn load_icon(path: &std::path::Path) -> tao::system_tray::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tao::system_tray::Icon::from_rgba(icon_rgba, icon_width, icon_height)
        .expect("Failed to open icon")
}
