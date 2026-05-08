use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use bridge::handle::BackendHandle;
use bridge::message::MessageToBackend;
use bridge::modal_action::ModalAction;
use gpui::{prelude::*, *};
use gpui_component::{button::{Button, ButtonVariants}, h_flex, v_flex, table::{Column, ColumnSort, TableDelegate}, Sizable, ActiveTheme};
use crate::{icon::PandoraIcon, modals::delete_server::open_delete_server, root, ts, ui};

#[derive(Clone, Debug)]
pub struct ServerEntry {
    pub name: SharedString,
    pub software: SharedString,
    pub version: SharedString,
    pub path: PathBuf,
    pub status: ServerStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running,
}

pub struct ServerList {
    pub items: Vec<ServerEntry>,
    columns: Vec<Column>,
    backend_handle: BackendHandle,
    running_servers: Arc<Mutex<std::collections::HashMap<String, bool>>>,
}

impl ServerList {
    pub fn new(items: Vec<ServerEntry>, backend_handle: BackendHandle) -> Self {
        Self {
            items,
            columns: vec![
                Column::new("controls", "").width(200.).fixed_left().resizable(true).movable(false),
                Column::new("name", "Name").width(150.).fixed_left().sortable().resizable(true).movable(false),
                Column::new("version", "Version").width(120.).fixed_left().sortable().resizable(true).movable(false),
                Column::new("software", "Software").width(100.).fixed_left().resizable(true).movable(false),
            ],
            backend_handle,
            running_servers: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Reload the server list items from disk
    pub fn reload_from_disk(&mut self) {
        let pandora_dir = if let Ok(dir) = std::env::var("PANDORA_DIR") {
            PathBuf::from(dir)
        } else {
            let base_dirs = directories::BaseDirs::new().unwrap();
            let data_dir = base_dirs.data_dir();
            data_dir.join("PandoraLauncher")
        };
        let servers_dir = pandora_dir.join("servers");

        let mut items = Vec::new();
        if servers_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&servers_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let metadata_path = path.join("server_metadata.json");
                        if let Ok(content) = std::fs::read_to_string(&metadata_path) {
                            if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&content) {
                                let name = metadata["name"].as_str().unwrap_or("Unknown").to_string();
                                let software = metadata["server_software"].as_str().unwrap_or("Paper").to_string();
                                let version = metadata["minecraft_version"].as_str().unwrap_or("1.20").to_string();
                                
                                // Check if server is marked as running in our tracking map
                                let is_running = self.running_servers.lock().ok()
                                    .and_then(|map| map.get(&name).copied())
                                    .unwrap_or(false);
                                
                                // Also verify the server process actually exists
                                let server_process_exists = {
                                    let pid_file = path.join("server.pid");
                                    if let Ok(pid_str) = std::fs::read_to_string(&pid_file) {
                                        if let Ok(pid) = pid_str.trim().parse::<u32>() {
                                            // Check if process exists on Linux
                                            #[cfg(unix)]
                                            {
                                                std::path::Path::new(&format!("/proc/{}", pid)).exists()
                                            }
                                            #[cfg(not(unix))]
                                            {
                                                true // On non-Linux, assume it exists if pid file exists
                                            }
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                };
                                
                                let status = if is_running && server_process_exists {
                                    ServerStatus::Running
                                } else {
                                    // Process no longer exists, update our map
                                    if is_running && !server_process_exists {
                                        if let Ok(mut map) = self.running_servers.lock() {
                                            map.insert(name.clone(), false);
                                        }
                                    }
                                    ServerStatus::Stopped
                                };
                                
                                items.push(ServerEntry {
                                    name: SharedString::from(name),
                                    software: SharedString::from(software),
                                    version: SharedString::from(version),
                                    path: path.clone(),
                                    status,
                                });
                            }
                        }
                    }
                }
            }
        }
        self.items = items;
    }

    pub fn create_table(data: &crate::entity::DataEntities, window: &mut Window, cx: &mut App) -> Entity<gpui_component::table::TableState<Self>> {
        let pandora_dir = if let Ok(dir) = std::env::var("PANDORA_DIR") {
            PathBuf::from(dir)
        } else {
            let base_dirs = directories::BaseDirs::new().unwrap();
            let data_dir = base_dirs.data_dir();
            data_dir.join("PandoraLauncher")
        };
        let servers_dir = pandora_dir.join("servers");

        let mut items = Vec::new();
        if servers_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&servers_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let metadata_path = path.join("server_metadata.json");
                        if let Ok(content) = std::fs::read_to_string(&metadata_path) {
                            if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&content) {
                                let name = metadata["name"].as_str().unwrap_or("Unknown").to_string();
                                let software = metadata["server_software"].as_str().unwrap_or("Paper").to_string();
                                let version = metadata["minecraft_version"].as_str().unwrap_or("1.20").to_string();
                                
                                items.push(ServerEntry {
                                    name: SharedString::from(name),
                                    software: SharedString::from(software),
                                    version: SharedString::from(version),
                                    path: path.clone(),
                                    status: ServerStatus::Stopped,
                                });
                            }
                        }
                    }
                }
            }
        }

        cx.new(|cx| {
            let server_list = Self::new(items, data.backend_handle.clone());
            gpui_component::table::TableState::new(server_list, window, cx)
        })
    }

    pub fn mark_server_running(&mut self, name: &str) {
        if let Ok(mut map) = self.running_servers.lock() {
            map.insert(name.to_string(), true);
        }
        if let Some(item) = self.items.iter_mut().find(|i| i.name.as_str() == name) {
            item.status = ServerStatus::Running;
        }
    }

    pub fn mark_server_stopped(&mut self, name: &str) {
        if let Ok(mut map) = self.running_servers.lock() {
            map.insert(name.to_string(), false);
        }
        if let Some(item) = self.items.iter_mut().find(|i| i.name.as_str() == name) {
            item.status = ServerStatus::Stopped;
        }
    }

    pub fn update_server_stopped(&self, name: &str) {
        if let Ok(mut map) = self.running_servers.lock() {
            map.insert(name.to_string(), false);
        }
    }

    pub fn render_card(&self, index: usize, cx: &mut App) -> Div {
        let item = &self.items[index];
        
        let software_and_version = format!(
            "{} {}",
            item.software.as_str(),
            item.version.as_str(),
        );

        // Check if server is running according to our map (true source of truth)
        let is_running = self.running_servers
            .lock()
            .ok()
            .and_then(|map| map.get(item.name.as_str()).copied())
            .unwrap_or(false);

        let status_button = if is_running {
            // Server is running, show Kill button
            Button::new(("kill_server", index))
                .info()
                .small()
                .flex_1()
                .label("Kill")
                .on_click({
                    let name = item.name.clone();
                    let backend_handle = self.backend_handle.clone();
                    let running_servers = self.running_servers.clone();
                    move |_, _, _| {
                        if let Ok(mut map) = running_servers.lock() {
                            map.insert(name.as_str().to_string(), false);
                        }
                        backend_handle.send(MessageToBackend::SendServerCommand {
                            name: name.as_str().into(),
                            command: "stop".into(),
                        });
                    }
                })
        } else {
            // Server is not running, show Start button
            match item.status {
                ServerStatus::Starting => {
                    Button::new(("launching", index))
                        .warning()
                        .small()
                        .flex_1()
                        .label("...")
                },
                _ => {
                    Button::new(("start_server", index))
                        .success()
                        .small()
                        .flex_1()
                        .label("Start")
                        .on_click({
                            let name = item.name.clone();
                            let backend_handle = self.backend_handle.clone();
                            let running_servers = self.running_servers.clone();
                            move |_, _, _| {
                                if let Ok(mut map) = running_servers.lock() {
                                    map.insert(name.as_str().to_string(), true);
                                }
                                backend_handle.send(MessageToBackend::StartServer {
                                    name: name.as_str().into(),
                                    modal_action: ModalAction::default(),
                                });
                            }
                        })
                }
            }
        };

        let view_button = Button::new(("view", index))
            .info()
            .small()
            .icon(PandoraIcon::Eye)
            .label(ts!("instance.view"))
            .on_click({
                let name = item.name.clone();
                move |_, window, cx| {
                    root::switch_page(
                        ui::PageType::ServerPage { name: name.clone() },
                        &[ui::PageType::Servers],
                        window,
                        cx,
                    );
                }
            });

        let open_folder_button = Button::new(("open_folder", index))
            .info()
            .small()
            .icon(PandoraIcon::FolderOpen)
            .on_click({
                let path = item.path.clone();
                move |_, window, cx| {
                    crate::open_folder(&path, window, cx);
                }
            });

        let theme = cx.theme();
        v_flex()
            .flex_1()
            .p_2()
            .gap_2()
            .w_full()
            .min_w_64()
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius_lg)
            .child(h_flex()
                .w_full()
                .gap_2()
                .child(v_flex()
                    .truncate()
                    .w_full()
                    .child(item.name.clone())
                    .child(software_and_version)
                )
            )
            .child(h_flex()
                .gap_2()
                .child(status_button.flex_1())
                .child(view_button.flex_1())
                .child(open_folder_button.flex_1())
                .child(Button::new(("delete", index))
                    .flex_1()
                    .small()
                    .info()
                    .icon(PandoraIcon::Trash2)
                    .on_click({
                        let backend_handle = self.backend_handle.clone();
                        let server_name = item.name.clone();
                        move |_, window, cx| {
                            open_delete_server(server_name.clone(), backend_handle.clone(), window, cx);
                        }
                    })
                )
            )
    }
}

impl TableDelegate for ServerList {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.items.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _window: &mut Window,
        cx: &mut Context<gpui_component::table::TableState<Self>>,
    ) {
        if let Some(col) = self.columns.get_mut(col_ix) {
            match col.key.as_ref() {
                "name" => self.items.sort_by(|a, b| match sort {
                    ColumnSort::Descending => lexical_sort::natural_lexical_cmp(&a.name, &b.name).reverse(),
                    _ => lexical_sort::natural_lexical_cmp(&a.name, &b.name),
                }),
                "version" => self.items.sort_by(|a, b| match sort {
                    ColumnSort::Descending => lexical_sort::natural_lexical_cmp(&a.version, &b.version).reverse(),
                    _ => lexical_sort::natural_lexical_cmp(&a.version, &b.version),
                }),
                _ => {}
            }
        }
        cx.notify();
    }

    fn render_td(&mut self, row_ix: usize, col_ix: usize, _window: &mut Window, _cx: &mut Context<gpui_component::table::TableState<Self>>) -> impl IntoElement {
        let item = self.items[row_ix].clone();

        if let Some(col) = self.columns.get(col_ix) {
            match col.key.as_ref() {
                "controls" => {
                    // Check if server is running according to our map (true source of truth)
                    let is_running = self.running_servers
                        .lock()
                        .ok()
                        .and_then(|map| map.get(item.name.as_str()).copied())
                        .unwrap_or(false);

                    let status_button = if is_running {
                        // Server is running, show Kill button
                        Button::new(("kill_server", row_ix))
                            .info()
                            .small()
                            .w(px(50.0))
                            .label("Kill")
                            .on_click({
                                let name = item.name.clone();
                                let backend_handle = self.backend_handle.clone();
                                let running_servers = self.running_servers.clone();
                                move |_, _, _| {
                                    if let Ok(mut map) = running_servers.lock() {
                                        map.insert(name.as_str().to_string(), false);
                                    }
                                    backend_handle.send(MessageToBackend::SendServerCommand {
                                        name: name.as_str().into(),
                                        command: "stop".into(),
                                    });
                                }
                            })
                    } else {
                        // Server is not running, show Start button
                        match item.status {
                            ServerStatus::Starting => {
                                Button::new(("launching", row_ix))
                                    .warning()
                                    .small()
                                    .w(px(50.0))
                                    .label("...")
                            },
                            _ => {
                                Button::new(("start_server", row_ix))
                                    .success()
                                    .small()
                                    .w(px(50.0))
                                    .label("Start")
                                    .on_click({
                                        let name = item.name.clone();
                                        let backend_handle = self.backend_handle.clone();
                                        let running_servers = self.running_servers.clone();
                                        move |_, _, _| {
                                            if let Ok(mut map) = running_servers.lock() {
                                                map.insert(name.as_str().to_string(), true);
                                            }
                                            backend_handle.send(MessageToBackend::StartServer {
                                                name: name.as_str().into(),
                                                modal_action: ModalAction::default(),
                                            });
                                        }
                                    })
                            }
                        }
                    };

                    let open_folder_button = Button::new(("open_folder", row_ix))
                        .info()
                        .small()
                        .icon(PandoraIcon::Folder)
                        .on_click({
                            let path = item.path.clone();
                            move |_, window, cx| {
                                crate::open_folder(&path, window, cx);
                            }
                        });

                    let delete_button = Button::new(("delete", row_ix))
                        .info()
                        .small()
                        .icon(PandoraIcon::Trash2)
                        .on_click({
                            let backend_handle = self.backend_handle.clone();
                            let server_name = item.name.clone();
                            move |_, window, cx| {
                                open_delete_server(server_name.clone(), backend_handle.clone(), window, cx);
                            }
                        });

                    h_flex()
                        .gap_2()
                        .size_full()
                        .px_2()
                        .child(status_button)
                        .child(Button::new(("view", row_ix)).small().info().w(px(50.0)).label(ts!("instance.view")).on_click({
                            let name = item.name.clone();
                            move |_, window, cx| {
                                root::switch_page(
                                    ui::PageType::ServerPage { name: name.clone() },
                                    &[ui::PageType::Servers],
                                    window,
                                    cx,
                                );
                            }
                        }))
                        .child(open_folder_button)
                        .child(delete_button)
                        .into_any_element()
                },
                "name" => item.name.clone().into_any_element(),
                "version" => item.version.clone().into_any_element(),
                "software" => h_flex()
                    .size_full()
                    .border_r_4()
                    .px_2()
                    .child(item.software.clone())
                    .into_any_element(),
                _ => div().into_any_element(),
            }
        } else {
            div().into_any_element()
        }
    }
}

