use std::{collections::HashMap, rc::Rc, sync::{Arc, atomic::AtomicBool}};

use bridge::{instance::InstanceStatus, keep_alive::KeepAliveHandle, message::{BridgeNotificationType, MessageToFrontend}};
use gpui::{AnyWindowHandle, App, AppContext, Bounds, Entity, Point, SharedString, Size, TitlebarOptions, Window, WindowBounds, WindowDecorations, WindowHandle, WindowOptions, px, size};
use gpui_component::{notification::{Notification, NotificationType}, Root, WindowExt};

use crate::{entity::{DataEntities, account::AccountEntries, instance::InstanceEntries, metadata::FrontendMetadata}, game_output::{GameOutput, GameOutputRoot}, interface_config::InterfaceConfig, root::LauncherRoot, ts};
use crate::game_output::ScrollHandler;

pub struct Processor {
    data: DataEntities,
    game_output_window: Option<WindowHandle<Root>>,
    game_output_tabs: HashMap<usize, Entity<GameOutput>>,
    game_output_names: HashMap<usize, SharedString>,
    game_output_keep_alives: HashMap<usize, KeepAliveHandle>,
    game_output_instance_ids: HashMap<usize, bridge::instance::InstanceID>,
    game_output_root: Option<Entity<GameOutputRoot>>,
    main_window_handle: Option<AnyWindowHandle>,
    main_window_hidden: Arc<AtomicBool>,
    waiting_for_window: Vec<MessageToFrontend>,
}

impl Processor {
    pub fn new(data: DataEntities, main_window_hidden: Arc<AtomicBool>) -> Self {
        Self {
            data,
            game_output_window: None,
            game_output_tabs: HashMap::new(),
            game_output_names: HashMap::new(),
            game_output_keep_alives: HashMap::new(),
            game_output_instance_ids: HashMap::new(),
            game_output_root: None,
            main_window_handle: None,
            main_window_hidden,
            waiting_for_window: Vec::new(),
        }
    }

    pub fn set_main_window_handle(&mut self, window: AnyWindowHandle, cx: &mut App) {
        self.main_window_handle = Some(window);
        self.process_messages_waiting_for_window(cx);
    }

    pub fn process_messages_waiting_for_window(&mut self, cx: &mut App) {
        for message in std::mem::take(&mut self.waiting_for_window) {
            self.process(message, cx);
        }
    }

    #[inline(always)]
    pub fn with_main_window(&mut self, message: MessageToFrontend, cx: &mut App, func: impl FnOnce(&mut Processor, MessageToFrontend, &mut Window, &mut App)) {
        let Some(handle) = self.main_window_handle else {
            self.waiting_for_window.push(message);
            return;
        };

        _ = handle.update(cx, |_, window, cx| {
            (func)(self, message, window, cx);
        });
    }

    pub fn process(&mut self, message: MessageToFrontend, cx: &mut App) {
        match message {
            MessageToFrontend::AccountsUpdated {
                accounts,
                selected_account,
            } => {
                AccountEntries::set(&self.data.accounts, accounts, selected_account, cx);
            },
            MessageToFrontend::InstanceAdded {
                id,
                name,
                icon,
                root_path,
                dot_minecraft_folder,
                configuration,
                worlds_state,
                servers_state,
                mods_state,
                resource_packs_state,
            } => {
                InstanceEntries::add(
                    &self.data.instances,
                    id,
                    name.as_str().into(),
                    icon,
                    root_path,
                    dot_minecraft_folder,
                    configuration,
                    worlds_state,
                    servers_state,
                    mods_state,
                    resource_packs_state,
                    cx,
                );
            },
            MessageToFrontend::InstanceRemoved { id } => {
                InstanceEntries::remove(&self.data.instances, id, cx);
            },
            MessageToFrontend::InstanceModified {
                id,
                name,
                icon,
                root_path,
                dot_minecraft_folder,
                configuration,
                status,
            } => {
                if status == InstanceStatus::Running {
                    if InterfaceConfig::get(cx).hide_main_window_on_launch {
                        if let Some(handle) = self.main_window_handle.take() {
                            self.main_window_hidden.store(true, std::sync::atomic::Ordering::SeqCst);
                            _ = handle.update(cx, |_, window, _| {
                                window.remove_window();
                            });
                        }
                    }
                } else if status == InstanceStatus::NotRunning {
                    if self.main_window_handle.is_none() && self.main_window_hidden.load(std::sync::atomic::Ordering::SeqCst) {
                        self.main_window_handle = Some(crate::open_main_window(&self.data, cx));
                        self.main_window_hidden.store(false, std::sync::atomic::Ordering::SeqCst);
                        self.process_messages_waiting_for_window(cx);
                    }
                }

                InstanceEntries::modify(
                    &self.data.instances,
                    id,
                    name.as_str().into(),
                    icon,
                    root_path,
                    dot_minecraft_folder,
                    configuration,
                    status,
                    cx,
                );
                
                // Notify GameOutputRoot of instance status change
                if let Some(root_entity) = &self.game_output_root {
                    root_entity.update(cx, |root, _| {
                        root.update_instance_status(id, status);
                    });
                }
            },
            MessageToFrontend::InstanceWorldsUpdated { id, worlds } => {
                InstanceEntries::set_worlds(&self.data.instances, id, worlds, cx);
            },
            MessageToFrontend::InstanceServersUpdated { id, servers } => {
                InstanceEntries::set_servers(&self.data.instances, id, servers, cx);
            },
            MessageToFrontend::InstanceModsUpdated { id, mods } => {
                InstanceEntries::set_mods(&self.data.instances, id, mods, cx);
            },
            MessageToFrontend::InstanceResourcePacksUpdated { id, resource_packs } => {
                InstanceEntries::set_resource_packs(&self.data.instances, id, resource_packs, cx);
            },
            MessageToFrontend::AddNotification { .. } => {
                self.with_main_window(message, cx, |_, message, window, cx| {
                    let MessageToFrontend::AddNotification { notification_type, message } = message else {
                        unreachable!();
                    };

                    let notification_type = match notification_type {
                        BridgeNotificationType::Success => NotificationType::Success,
                        BridgeNotificationType::Info => NotificationType::Info,
                        BridgeNotificationType::Error => NotificationType::Error,
                        BridgeNotificationType::Warning => NotificationType::Warning,
                    };
                    let mut notification: Notification = (notification_type, SharedString::from(message)).into();
                    if let NotificationType::Error = notification_type {
                        notification = notification.autohide(false);
                    }
                    window.push_notification(notification, cx);
                });
            },
            MessageToFrontend::Refresh => {
                let Some(handle) = self.main_window_handle else {
                    return;
                };
                _ = handle.update(cx, |_, window, _| {
                    window.refresh();
                });
            },
            MessageToFrontend::CloseModal => {
                let Some(handle) = self.main_window_handle else {
                    return;
                };
                _ = handle.update(cx, |_, window, cx| {
                    window.close_all_dialogs(cx);
                });
            },
            MessageToFrontend::CreateGameOutputWindow { id, name, keep_alive } => {
                let instance_name: SharedString = name.into();
                self.game_output_names.insert(id, instance_name.clone());
                let keep_alive_handle = keep_alive.create_handle();
                self.game_output_keep_alives.insert(id, keep_alive_handle.clone());
                
                // Find the instance ID by name
                if let Some(instance_id) = InstanceEntries::find_id_by_name(&self.data.instances, &instance_name, cx) {
                    self.game_output_instance_ids.insert(id, instance_id);
                }
                
                // Check if the window still exists - if not, clear the reference
                let window_still_exists = if let Some(window_handle) = &self.game_output_window {
                    window_handle.update(cx, |_, _, _| {}).is_ok()
                } else {
                    false
                };
                
                if !window_still_exists {
                    self.game_output_window = None;
                    self.game_output_root = None;
                }
                
                if self.game_output_window.is_none() {
                    // First instance - create the window
                    let window_bounds = match InterfaceConfig::get(cx).game_output_bounds {
                        crate::interface_config::WindowBounds::Inherit => None,
                        crate::interface_config::WindowBounds::Windowed { x, y, w, h } => {
                            Some(WindowBounds::Windowed(Bounds::new(Point::new(px(x), px(y)), Size::new(px(w), px(h)))))
                        },
                        crate::interface_config::WindowBounds::Maximized { x, y, w, h } => {
                            Some(WindowBounds::Maximized(Bounds::new(Point::new(px(x), px(y)), Size::new(px(w), px(h)))))
                        },
                        crate::interface_config::WindowBounds::Fullscreen { x, y, w, h } => {
                            Some(WindowBounds::Fullscreen(Bounds::new(Point::new(px(x), px(y)), Size::new(px(w), px(h)))))
                        },
                    };

                    let options = WindowOptions {
                        app_id: Some("PandoraLauncher".into()),
                        window_min_size: Some(size(px(360.0), px(240.0))),
                        window_bounds,
                        titlebar: Some(TitlebarOptions {
                            title: Some(ts!("system.game_output")),
                            ..Default::default()
                        }),
                        window_decorations: Some(WindowDecorations::Server),
                        ..Default::default()
                    };
                    
                    // Create initial GameOutput for this instance
                    let game_output = cx.new(|_| GameOutput::default());
                    self.game_output_tabs.insert(id, game_output.clone());
                    
                    let processor_ptr = self as *mut Processor;
                    let backend_handle = self.data.backend_handle.clone();
                    let instance_id = self.game_output_instance_ids.get(&id).copied();
                    let instance_root_path = if let Some(inst_id) = instance_id {
                        InstanceEntries::find_root_path_by_id(&self.data.instances, inst_id, cx)
                            .unwrap_or_else(|| Arc::from(std::path::Path::new(".")))
                    } else {
                        Arc::from(std::path::Path::new("."))
                    };
                    let dot_minecraft_path = if let Some(inst_id) = instance_id {
                        InstanceEntries::find_dot_minecraft_by_id(&self.data.instances, inst_id, cx)
                            .unwrap_or_else(|| Arc::from(std::path::Path::new(".")))
                    } else {
                        Arc::from(std::path::Path::new("."))
                    };
                    _ = cx.open_window(options, move |window, cx| {
                        let window_handle = window.window_handle().downcast::<Root>().unwrap();
                        
                        let game_output_root = cx.new(|cx| {
                            if let Some(inst_id) = instance_id {
                                GameOutputRoot::new_tabbed(id, inst_id, instance_name, instance_root_path.clone(), dot_minecraft_path.clone(), keep_alive, game_output, backend_handle.clone(), window, cx)
                            } else {
                                // Fallback if instance ID not found (shouldn't happen)
                                GameOutputRoot::new_tabbed(id, bridge::instance::InstanceID::dangling(), instance_name, instance_root_path.clone(), dot_minecraft_path.clone(), keep_alive, game_output, backend_handle.clone(), window, cx)
                            }
                        });
                        window.activate_window();
                        
                        // Store window handle and root entity in processor
                        unsafe {
                            (*processor_ptr).game_output_window = Some(window_handle);
                            (*processor_ptr).game_output_root = Some(game_output_root.clone());
                        }

                        cx.new(|cx| Root::new(game_output_root, window, cx))
                    });
                } else if !self.game_output_tabs.contains_key(&id) {
                    // Window exists but this is a new instance - add a tab
                    let game_output = cx.new(|_| GameOutput::default());
                    self.game_output_tabs.insert(id, game_output.clone());
                    
                    // Tell the GameOutputRoot to add this new tab
                    if let Some(root_entity) = &self.game_output_root {
                        if let Some(instance_id) = self.game_output_instance_ids.get(&id) {
                            let instance_root_path = InstanceEntries::find_root_path_by_id(&self.data.instances, *instance_id, cx)
                                .unwrap_or_else(|| Arc::from(std::path::Path::new(".")));
                            let dot_minecraft_path = InstanceEntries::find_dot_minecraft_by_id(&self.data.instances, *instance_id, cx)
                                .unwrap_or_else(|| Arc::from(std::path::Path::new(".")));
                            root_entity.update(cx, |root, cx| {
                                root.create_or_switch_tab(id, *instance_id, instance_name, instance_root_path, dot_minecraft_path, game_output, keep_alive_handle.clone(), cx);
                            });
                        }
                    }
                }
            },
            MessageToFrontend::AddGameOutput {
                id,
                time,
                level,
                text,
            } => {
                if let Some(game_output) = self.game_output_tabs.get(&id) {
                    game_output.update(cx, |game_output, _| {
                        game_output.add(time, level, text);
                    });
                    
                    // Switch to this instance's tab if it exists
                    if let Some(root_entity) = &self.game_output_root {
                        root_entity.update(cx, |root, cx| {
                            root.active_instance_id = Some(id);
                            if let Some(game_output_entity) = root.tabs.get(&id) {
                                root.game_output = game_output_entity.clone();
                                let scroll_state = Rc::clone(&root.game_output.read(cx).scroll_state);
                                root.scroll_handler = ScrollHandler { state: scroll_state };
                            }
                            cx.notify();
                        });
                    }
                    
                    // Refresh the window if it exists
                    if let Some(window_handle) = &self.game_output_window {
                        _ = window_handle.update(cx, |_, window, _cx| {
                            window.refresh();
                        });
                    }
                }
            },
            MessageToFrontend::MoveInstanceToTop { id } => {
                InstanceEntries::move_to_top(&self.data.instances, id, cx);
            },
            MessageToFrontend::MetadataResult { request, result, keep_alive_handle } => {
                FrontendMetadata::set(&self.data.metadata, request, result, keep_alive_handle, cx);
            },
            MessageToFrontend::SkinLibraryUpdated { skin_library } => {
                self.data.set_skin_library(skin_library, cx);
            },
            MessageToFrontend::UpdateAvailable { .. } => {
                self.with_main_window(message, cx, |_, message, window, cx| {
                    let MessageToFrontend::UpdateAvailable { update } = message else {
                        unreachable!();
                    };

                    if let Some(root) = window.root::<Root>().flatten() {
                        if let Ok(launcher_root) = root.read(cx).view().clone().downcast::<LauncherRoot>() {
                            launcher_root.update(cx, |launcher_root, cx| {
                                launcher_root.ui.update(cx, |ui, cx| {
                                    ui.update = Some(update);
                                    cx.notify();
                                });
                            });
                        }
                    }
                });
            }
        }
    }
}
