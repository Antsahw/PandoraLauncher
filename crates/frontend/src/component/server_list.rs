use std::path::PathBuf;
use bridge::handle::BackendHandle;
use bridge::message::MessageToBackend;
use bridge::modal_action::ModalAction;
use gpui::{prelude::*, *};
use gpui_component::{button::{Button, ButtonVariants}, h_flex, v_flex, table::{Column, TableDelegate}, Sizable, ActiveTheme};
use crate::{icon::PandoraIcon, root, ts, ui};

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
    backend_handle: BackendHandle,
}

impl ServerList {
    pub fn new(items: Vec<ServerEntry>, backend_handle: BackendHandle) -> Self {
        Self {
            items,
            backend_handle,
        }
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
            let server_list = Self {
                items,
                backend_handle: data.backend_handle.clone(),
            };
            gpui_component::table::TableState::new(server_list, window, cx)
        })
    }

    pub fn render_card(&self, index: usize, cx: &mut App) -> Div {
        let item = &self.items[index];
        
        let software_and_version = format!(
            "{} {}",
            item.software.as_str(),
            item.version.as_str(),
        );

        let status_button = match item.status {
            ServerStatus::Stopped => {
                Button::new(format!("start_server_{}", item.name))
                    .success()
                    .small()
                    .label("Start")
                    .on_click({
                        let name = item.name.clone();
                        let backend_handle = self.backend_handle.clone();
                        move |_, _, _| {
                            backend_handle.send(MessageToBackend::StartServer {
                                name: name.as_str().into(),
                                modal_action: ModalAction::default(),
                            });
                        }
                    })
            },
            ServerStatus::Starting => {
                Button::new(format!("launching_server_{}", item.name))
                    .small()
                    .label("Starting...")
            },
            ServerStatus::Running => {
                Button::new(format!("stop_server_{}", item.name))
                    .danger()
                    .small()
                    .label("Stop")
                    .on_click({
                        let name = item.name.clone();
                        let backend_handle = self.backend_handle.clone();
                        move |_, _, _| {
                            backend_handle.send(MessageToBackend::StopServer {
                                name: name.as_str().into(),
                            });
                        }
                    })
            },
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
                .child(view_button.flex_1())
                .child(status_button.flex_1())
            )
    }

}

impl TableDelegate for ServerList {
    fn columns_count(&self, _cx: &App) -> usize {
        4
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.items.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        match col_ix {
            0 => Column::new("controls", "").width(150.).fixed_left().movable(false).resizable(false),
            1 => Column::new("name", "Name").width(150.).fixed_left().sortable().resizable(true),
            2 => Column::new("software", "Software").width(100.).fixed_left().resizable(true),
            3 => Column::new("version", "Version").width(120.).fixed_left().sortable().resizable(true),
            _ => Column::new("unknown", "").width(100.),
        }
    }

    fn render_td(&mut self, row_ix: usize, col_ix: usize, _window: &mut Window, _cx: &mut Context<gpui_component::table::TableState<Self>>) -> impl IntoElement {
        let item = &mut self.items[row_ix];

        match col_ix {
            0 => {
                let status_button = match item.status {
                    ServerStatus::Stopped => {
                        Button::new(format!("start_server_{}", item.name))
                            .success()
                            .small()
                            .label("Start")
                            .on_click({
                                let name = item.name.clone();
                                let backend_handle = self.backend_handle.clone();
                                move |_, _, _| {
                                    // Send start command to backend
                                    backend_handle.send(MessageToBackend::StartServer {
                                        name: name.as_str().into(),
                                        modal_action: ModalAction::default(),
                                    });
                                }
                            })
                    },
                    ServerStatus::Starting => {
                        Button::new(format!("launching_server_{}", item.name))
                            .small()
                            .label("Starting...")
                    },
                    ServerStatus::Running => {
                        Button::new(format!("stop_server_{}", item.name))
                            .danger()
                            .small()
                            .label("Stop")
                            .on_click({
                                let name = item.name.clone();
                                let backend_handle = self.backend_handle.clone();
                                move |_, _, _| {
                                    // Send stop command to backend
                                    backend_handle.send(MessageToBackend::StopServer {
                                        name: name.as_str().into(),
                                    });
                                }
                            })
                    },
                };

                let view_button = Button::new(format!("view_server_{}", item.name))
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

                h_flex()
                    .gap_2()
                    .child(view_button)
                    .child(status_button)
                    .into_any_element()
            }
            1 => div().child(item.name.clone()).into_any_element(),
            2 => div().child(item.software.clone()).into_any_element(),
            3 => div().child(item.version.clone()).into_any_element(),
            _ => div().into_any_element(),
        }
    }
}
