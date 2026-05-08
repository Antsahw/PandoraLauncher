use bridge::handle::BackendHandle;
use bridge::message::MessageToBackend;
use gpui::{prelude::*, *, Subscription};
use gpui_component::{
    ActiveTheme as _, WindowExt, Sizable, button::{Button, ButtonVariants}, h_flex, input::{Input, InputEvent, InputState}, tab::{Tab, TabBar}, v_flex, spinner::Spinner, select::{Select, SelectEvent, SelectState}, scroll::ScrollableElement
};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};

use crate::{
    entity::DataEntities, component::{named_dropdown::{NamedDropdown, NamedDropdownItem}, readonly_text_field::{ReadonlyTextField, ReadonlyTextFieldWithControls}}, icon::PandoraIcon, interface_config::InterfaceConfig, pages::{instance::{server_settings_subpage::ServerSettingsSubpage, server_statistics_subpage::ServerStatisticsSubpage}, page::Page}, root, ts
};

pub struct ServerPage {
    backend_handle: BackendHandle,
    _data: DataEntities,
    server_name: SharedString,
    subpage: ServerSubpage,
}

impl ServerPage {
    pub fn new(
        server_name: SharedString,
        data: &DataEntities,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let subpage_type = InterfaceConfig::get(cx).server_subpage;
        let subpage = subpage_type.create(server_name.clone(), data.clone(), data.backend_handle.clone(), window, cx);

        Self {
            backend_handle: data.backend_handle.clone(),
            _data: data.clone(),
            server_name,
            subpage,
        }
    }
}

impl Page for ServerPage {
    fn controls(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let backend_handle = self.backend_handle.clone();
        let server_name = self.server_name.clone();

        h_flex()
            .gap_3()
            .child(
                Button::new("back_to_servers")
                    .icon(PandoraIcon::ChevronLeft)
                    .label("Back")
                    .on_click(move |_, window, cx| {
                        root::switch_page(
                            crate::ui::PageType::Servers,
                            &[],
                            window,
                            cx,
                        );
                    }),
            )
            .child(
                Button::new("rename_server")
                    .label("Rename")
                    .on_click({
                        let server_name = server_name.clone();
                        let backend_handle = backend_handle.clone();
                        move |_, window, cx| {
                            // Create the input state in the &mut App context BEFORE opening dialog
                            let input_entity = cx.new(|cx| {
                                gpui_component::input::InputState::new(window, cx)
                            });
                            
                            let backend_handle_clone = backend_handle.clone();
                            let server_name_clone = server_name.clone();
                            window.open_dialog(cx, move |dialog, _, _| {
                                dialog
                                    .title("Rename Server")
                                    .overlay_closable(false)
                                    .flex()
                                    .line_height(rems(1.2))
                                    .child("Enter new server name:")
                                    .child(gpui_component::input::Input::new(&input_entity))
                                    .footer(h_flex()
                                        .gap_2()
                                        .w_full()
                                        .child(
                                            Button::new("cancel_rename")
                                                .label("Cancel")
                                                .on_click(|_, window, cx| {
                                                    window.close_dialog(cx);
                                                }).flex_grow()
                                        )
                                        .child(
                                            Button::new("confirm_rename")
                                                .label("Rename")
                                                .on_click({
                                                    let backend_handle = backend_handle_clone.clone();
                                                    let server_name = server_name_clone.clone();
                                                    let input_entity = input_entity.clone();
                                                    move |_, window, cx| {
                                                        let new_name = input_entity.read(cx).value().clone();
                                                        if !new_name.is_empty() && new_name.as_str() != server_name.as_str() {
                                                            let _ = backend_handle.send(bridge::message::MessageToBackend::RenameServer {
                                                                old_name: server_name.as_str().into(),
                                                                new_name: new_name.as_str().into(),
                                                            });
                                                            window.close_dialog(cx);
                                                            root::switch_page(
                                                                crate::ui::PageType::Servers,
                                                                &[],
                                                                window,
                                                                cx,
                                                            );
                                                        }
                                                    }
                                                })
                                        ))
                            })
                        }
                    }),
            )
            .child(
                Button::new("delete_server")
                    .icon(PandoraIcon::Close)
                    .label("Delete")
                    .on_click({
                        let backend_handle = backend_handle.clone();
                        let server_name = server_name.clone();
                        move |_, window, cx| {
                            let backend_handle_clone = backend_handle.clone();
                            let server_name_clone = server_name.clone();
                            window.open_dialog(cx, move |dialog, _, _| {
                                dialog
                                    .title("Delete Server")
                                    .overlay_closable(false)
                                    .flex()
                                    .line_height(rems(1.2))
                                    .child("Are you sure you want to permanently delete this server?")
                                    .child("This will remove all server files and cannot be undone.")
                                    .footer(h_flex()
                                        .gap_2()
                                        .w_full()
                                        .child(
                                            Button::new("cancel_delete")
                                                .label("Cancel")
                                                .on_click(|_, window, cx| {
                                                    window.close_dialog(cx);
                                                }).flex_grow()
                                        )
                                        .child(
                                            Button::new("confirm_delete")
                                                .danger()
                                                .label("Delete Server")
                                                .on_click({
                                                    let backend_handle = backend_handle_clone.clone();
                                                    let server_name = server_name_clone.clone();
                                                    move |_, window, cx| {
                                                        let _ = backend_handle.send(bridge::message::MessageToBackend::DeleteServer {
                                                            name: server_name.as_str().into(),
                                                        });
                                                        window.close_dialog(cx);
                                                        root::switch_page(
                                                            crate::ui::PageType::Servers,
                                                            &[],
                                                            window,
                                                            cx,
                                                        );
                                                    }
                                                })
                                        ))
                            })
                        }
                    }),
            )
    }

    fn scrollable(&self, _cx: &App) -> bool {
        false
    }
}

impl Render for ServerPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let subpage_type = InterfaceConfig::get(cx).server_subpage;
        if subpage_type != self.subpage.page_type() {
            self.subpage = subpage_type.create(self.server_name.clone(), self._data.clone(), self.backend_handle.clone(), window, cx);
        }

        let selected_index = match &self.subpage {
            ServerSubpage::Logs(_) => 0,
            ServerSubpage::Settings(_) => 1,
            ServerSubpage::Properties(_) => 2,
            ServerSubpage::Statistics(_) => 3,
        };

        v_flex()
            .size_full()
            .child(
                TabBar::new("server_bar")
                    .prefix(div().w_4())
                    .selected_index(selected_index)
                    .underline()
                    .child(Tab::new().label(ts!("instance.logs.title")))
                    .child(Tab::new().label(ts!("settings.title")))
                    .child(Tab::new().label("Properties"))
                    .child(Tab::new().label(ts!("server.statistics")))
                    .on_click(cx.listener(|_, index, _, cx| {
                        let page_type = match *index {
                            0 => ServerSubpageType::Logs,
                            1 => ServerSubpageType::Settings,
                            2 => ServerSubpageType::Properties,
                            3 => ServerSubpageType::Statistics,
                            _ => {
                                return;
                            },
                        };
                        InterfaceConfig::get_mut(cx).server_subpage = page_type;
                    })),
            )
            .child(self.subpage.clone().into_any_element())
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerSubpageType {
    #[default]
    Logs,
    Settings,
    Properties,
    Statistics,
}

impl ServerSubpageType {
    pub fn create(
        self,
        server_name: SharedString,
        data: DataEntities,
        backend_handle: BackendHandle,
        window: &mut Window,
        cx: &mut App,
    ) -> ServerSubpage {
        match self {
            ServerSubpageType::Logs => ServerSubpage::Logs(cx.new(|cx| {
                ServerLogsSubpage::new(server_name.clone(), backend_handle, window, cx)
            })),
            ServerSubpageType::Settings => {
                ServerSubpage::Settings(cx.new(|cx| {
                    ServerSettingsSubpage::new(server_name.clone(), backend_handle.clone(), data, window, cx)
                }))
            }
            ServerSubpageType::Properties => {
                ServerSubpage::Properties(cx.new(|cx| {
                    ServerPropertiesSubpage::new(server_name.clone(), backend_handle, data, window, cx)
                }))
            }
            ServerSubpageType::Statistics => {
                ServerSubpage::Statistics(cx.new(|cx| {
                    ServerStatisticsSubpage::new(server_name.clone(), backend_handle, window, cx)
                }))
            }
        }
    }
}

#[derive(Clone)]
pub enum ServerSubpage {
    Logs(Entity<ServerLogsSubpage>),
    Settings(Entity<ServerSettingsSubpage>),
    Properties(Entity<ServerPropertiesSubpage>),
    Statistics(Entity<ServerStatisticsSubpage>),
}

impl ServerSubpage {
    pub fn page_type(&self) -> ServerSubpageType {
        match self {
            ServerSubpage::Logs(_) => ServerSubpageType::Logs,
            ServerSubpage::Settings(_) => ServerSubpageType::Settings,
            ServerSubpage::Properties(_) => ServerSubpageType::Properties,
            ServerSubpage::Statistics(_) => ServerSubpageType::Statistics,
        }
    }

    pub fn into_any_element(self) -> AnyElement {
        match self {
            Self::Logs(entity) => entity.into_any_element(),
            Self::Settings(entity) => entity.into_any_element(),
            Self::Properties(entity) => entity.into_any_element(),
            Self::Statistics(entity) => entity.into_any_element(),
        }
    }
}

// Server Logs Subpage
pub struct ServerLogsSubpage {
    server_name: SharedString,
    backend_handle: BackendHandle,
    log_content: Option<Entity<ReadonlyTextFieldWithControls>>,
    no_available_logs: bool,
    available_logs: Option<Entity<SelectState<NamedDropdown<Arc<Path>>>>>,
    clean_old_logs_text: Option<SharedString>,
    last_selected_path: Option<Arc<Path>>,
    _read_log_task: Option<Task<()>>,
    _get_log_files_task: Task<()>,
    _dropdown_change_subscrption: Option<Subscription>,
}

impl ServerLogsSubpage {
    pub fn new(_server_name: SharedString, _backend_handle: BackendHandle, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            server_name: _server_name,
            backend_handle: _backend_handle,
            log_content: None,
            no_available_logs: false,
            available_logs: None,
            clean_old_logs_text: None,
            last_selected_path: None,
            _read_log_task: None,
            _get_log_files_task: Task::ready(()),
            _dropdown_change_subscrption: None,
        };

        this.get_log_files(_window, _cx);

        this
    }
}

impl ServerLogsSubpage {
    pub fn get_log_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.no_available_logs = false;
        self.log_content = None;
        self.available_logs = None;
        self.clean_old_logs_text = None;
        self.last_selected_path = None;
        self._read_log_task = None;
        self._dropdown_change_subscrption = None;

        let (send, recv) = tokio::sync::oneshot::channel();
        self._get_log_files_task = cx.spawn_in(window, async move |page, cx| {
            let result: bridge::message::LogFiles = recv.await.unwrap_or_else(|_| bridge::message::LogFiles { paths: Vec::new(), total_gzipped_size: 0 });
            let _ = page.update_in(cx, move |page, window, cx| {
                if result.paths.is_empty() {
                    page.no_available_logs = true;
                } else {
                    let items = result.paths.into_iter().filter_map(|path| {
                        Some(NamedDropdownItem {
                            name: SharedString::new(Arc::from(path.file_name()?.to_string_lossy())),
                            item: path,
                        })
                    }).collect();

                    let dropdown = NamedDropdown::create(items, window, cx);

                    let _dropdown_change_subscrption = cx.subscribe_in(&dropdown, window, move |page, entity, _: &SelectEvent<NamedDropdown<Arc<Path>>>, window, cx| {
                        let selected = entity.read(cx).selected_value().map(|item| item.item.clone());

                        if selected == page.last_selected_path {
                            return;
                        }
                        page.last_selected_path = selected.clone();

                        if let Some(selected) = selected {
                            let (send, mut recv) = tokio::sync::mpsc::channel::<Arc<str>>(256);

                            let text_field = cx.new(move |_| ReadonlyTextField::default());

                            let text_field2 = text_field.clone();
                            page._read_log_task = Some(cx.spawn(async move |_, cx| {
                                while let Some(message) = recv.recv().await {
                                    let _ = cx.update_entity(&text_field2, |text_field, _| {
                                        text_field.add(message);
                                    });
                                }
                                let _ = cx.update_entity(&text_field2, |text_field, _| {
                                    text_field.shrink_to_fit();
                                });
                            }));

                            page.backend_handle.send(MessageToBackend::ReadLog {
                                path: selected.clone(),
                                send,
                            });

                            let backend_handle = page.backend_handle.clone();
                            page.log_content = Some(cx.new(move |cx| {
                                ReadonlyTextFieldWithControls::new(text_field, Box::new(move |div| {
                                    let backend_handle = backend_handle.clone();
                                    let selected = selected.clone();
                                    div.child(Button::new("upload").label(ts!("instance.logs.upload.label")).on_click(move |_, window, cx| {
                                        root::upload_log_file(selected.clone(), &backend_handle, window, cx);
                                    }))
                                }), window, cx)
                            }));
                        } else {
                            page._read_log_task = None;
                            page.log_content = None;
                        }

                        cx.notify();
                    });

                    page._dropdown_change_subscrption = Some(_dropdown_change_subscrption);
                    page.available_logs = Some(dropdown);

                    if result.total_gzipped_size > 0 {
                        let bytes = result.total_gzipped_size;
                        let string = if bytes < 1000 {
                            ts!("instance.logs.cleanup", num = format!("{} bytes", bytes))
                        } else if bytes < 1000*1000 {
                            ts!("instance.logs.cleanup", num = format!("{}kB", bytes/1000))
                        } else if bytes < 1000*1000*1000 {
                            ts!("instance.logs.cleanup", num = format!("{}MB", bytes/1000/1000))
                        } else {
                            ts!("instance.logs.cleanup", num = format!("{}GB", bytes/1000/1000/1000))
                        };
                        page.clean_old_logs_text = Some(string.into());
                    }
                }
                cx.notify();
            });
        });

        self.backend_handle.send(MessageToBackend::GetServerLogFiles {
            name: self.server_name.as_str().into(),
            channel: send,
        });
    }
}

impl Render for ServerLogsSubpage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let mut header = h_flex()
            .gap_3()
            .mb_1()
            .ml_1()
            .child(div().text_lg().child(ts!("instance.logs.title")));

        let mut content = div()
            .size_full()
            .border_1()
            .rounded(theme.radius)
            .border_color(theme.border);

        if self.no_available_logs {
            content = content.child(h_flex().justify_center().size_full().text_lg().child(ts!("instance.logs.none")));
        } else {
            if let Some(available_logs) = self.available_logs.as_ref() {
                header = header.child(Select::new(&available_logs).small().mt_0p5().placeholder(ts!("instance.logs.select_file")));
            } else {
                content = content.child(h_flex().justify_center().size_full().text_lg().gap_3().child(ts!("instance.logs.loading")).child(Spinner::new()));
            }

            if let Some(log_content) = self.log_content.clone() {
                content = content.child(log_content);
            } else if self.available_logs.is_some() {
                content = content.child(h_flex().justify_center().size_full().text_lg()
                    .gap_2()
                    .child(PandoraIcon::ArrowUp)
                    .child(ts!("instance.logs.select_file"))
                    .child(PandoraIcon::ArrowUp));
            }
        }

        if let Some(clean_old_logs_text) = self.clean_old_logs_text.clone() {
            header = header.child(Button::new("cleanold").label(clean_old_logs_text).success().compact().small().on_click({
                cx.listener(move |this, _, window, cx| {
                    // TODO: Implement server log cleanup in backend
                    // For now, just refresh the log files
                    this.get_log_files(window, cx);
                })
            }));
        }

        v_flex().p_4().size_full().child(header).child(content)
    }
}

// Property file types for server configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyFileType {
    ServerProperties,
    BannedPlayers,
    BannedIps,
    Whitelist,
    Ops,
    UserCache,
}

impl PropertyFileType {
    pub fn filename(&self) -> &'static str {
        match self {
            PropertyFileType::ServerProperties => "server.properties",
            PropertyFileType::BannedPlayers => "banned-players.json",
            PropertyFileType::BannedIps => "banned-ips.json",
            PropertyFileType::Whitelist => "whitelist.json",
            PropertyFileType::Ops => "ops.json",
            PropertyFileType::UserCache => "usercache.json",
        }
    }
    
    pub fn label(&self) -> &'static str {
        match self {
            PropertyFileType::ServerProperties => "Server Properties",
            PropertyFileType::BannedPlayers => "Bans",
            PropertyFileType::BannedIps => "IP Bans",
            PropertyFileType::Whitelist => "Whitelist",
            PropertyFileType::Ops => "Operators",
            PropertyFileType::UserCache => "User Cache",
        }
    }
}

// Server Properties Subpage - File editor for server config files
pub struct ServerPropertiesSubpage {
    server_name: SharedString,
    backend_handle: BackendHandle,
    data: DataEntities,
    current_file: PropertyFileType,
    
    // Separate InputState for each file
    server_props_input: Entity<InputState>,
    banned_players_input: Entity<InputState>,
    banned_ips_input: Entity<InputState>,
    whitelist_input: Entity<InputState>,
    ops_input: Entity<InputState>,
    user_cache_input: Entity<InputState>,
    
    search_input: Entity<InputState>,
    search_positions: Vec<(usize, usize)>,
    search_focused: bool,
    _file_input_subscriptions: Vec<Subscription>,
}

impl ServerPropertiesSubpage {
    pub fn new(server_name: SharedString, backend_handle: BackendHandle, data: DataEntities, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Create separate input for each file type
        let server_props_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Loading server.properties...")
                .auto_grow(10, 50)
        });
        
        let banned_players_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Loading banned-players.json...")
                .auto_grow(10, 50)
        });
        
        let banned_ips_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Loading banned-ips.json...")
                .auto_grow(10, 50)
        });
        
        let whitelist_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Loading whitelist.json...")
                .auto_grow(10, 50)
        });
        
        let ops_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Loading ops.json...")
                .auto_grow(10, 50)
        });
        
        let user_cache_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Loading usercache.json...")
                .auto_grow(10, 50)
        });
        
        let search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search (Ctrl+F)...")
        });
        
        // Subscription for file content changes
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe_in(&server_props_input, window, Self::on_file_content_changed));
        subscriptions.push(cx.subscribe_in(&banned_players_input, window, Self::on_file_content_changed));
        subscriptions.push(cx.subscribe_in(&banned_ips_input, window, Self::on_file_content_changed));
        subscriptions.push(cx.subscribe_in(&whitelist_input, window, Self::on_file_content_changed));
        subscriptions.push(cx.subscribe_in(&ops_input, window, Self::on_file_content_changed));
        subscriptions.push(cx.subscribe_in(&user_cache_input, window, Self::on_file_content_changed));
        subscriptions.push(cx.subscribe_in(&search_input, window, Self::on_search_event));
        
        // Request all files from backend
        for file_type in &[
            PropertyFileType::ServerProperties,
            PropertyFileType::BannedPlayers,
            PropertyFileType::BannedIps,
            PropertyFileType::Whitelist,
            PropertyFileType::Ops,
            PropertyFileType::UserCache,
        ] {
            backend_handle.send(bridge::message::MessageToBackend::ReadServerFile {
                name: server_name.as_str().into(),
                filename: file_type.filename().into(),
            });
        }
        
        Self {
            server_name,
            backend_handle,
            data,
            current_file: PropertyFileType::ServerProperties,
            server_props_input,
            banned_players_input,
            banned_ips_input,
            whitelist_input,
            ops_input,
            user_cache_input,
            search_input,
            search_positions: Vec::new(),
            search_focused: false,
            _file_input_subscriptions: subscriptions,
        }
    }
    
    fn get_input_for_file(&self, file_type: PropertyFileType) -> Entity<InputState> {
        match file_type {
            PropertyFileType::ServerProperties => self.server_props_input.clone(),
            PropertyFileType::BannedPlayers => self.banned_players_input.clone(),
            PropertyFileType::BannedIps => self.banned_ips_input.clone(),
            PropertyFileType::Whitelist => self.whitelist_input.clone(),
            PropertyFileType::Ops => self.ops_input.clone(),
            PropertyFileType::UserCache => self.user_cache_input.clone(),
        }
    }
    
    fn on_file_content_changed(
        &mut self,
        _state: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let input = self.get_input_for_file(self.current_file);
                let content = input.read(cx).value();
                
                self.backend_handle.send(bridge::message::MessageToBackend::WriteServerFile {
                    name: self.server_name.as_str().into(),
                    filename: self.current_file.filename().into(),
                    content: content.as_str().into(),
                });
            }
            InputEvent::Focus => {
                self.search_focused = false;
                cx.notify();
            }
            _ => {}
        }
    }
    
    fn on_search_event(
        &mut self,
        _state: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Focus => {
                self.search_focused = true;
                cx.notify();
            }
            InputEvent::Blur => {
                self.search_focused = false;
                cx.notify();
            }
            _ => {}
        }
    }
}

impl Render for ServerPropertiesSubpage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let file_types = vec![
            PropertyFileType::ServerProperties,
            PropertyFileType::BannedPlayers,
            PropertyFileType::BannedIps,
            PropertyFileType::Whitelist,
            PropertyFileType::Ops,
            PropertyFileType::UserCache,
        ];
        
        let selected_index = file_types.iter().position(|&f| f == self.current_file).unwrap_or(0);
        let backend_handle = self.backend_handle.clone();
        let server_name = self.server_name.clone();
        let data = self.data.clone();
        
        // Load content from cache into the appropriate InputState
        for file_type in &file_types {
            let input = self.get_input_for_file(*file_type);
            let cached_content = {
                let files = data.server_files.read();
                files.get(file_type.filename()).cloned()
            };
            
            if let Some(content) = cached_content {
                let current_value = input.read(cx).value();
                if current_value.starts_with("Loading") || current_value.is_empty() {
                    input.update(cx, |state, cx_inner| {
                        state.set_value(&content, window, cx_inner);
                    });
                }
            }
        }
        
        // Get current search term and find all matches
        let search_term = self.search_input.read(cx).value().to_string();
        let current_input = self.get_input_for_file(self.current_file);
        let content = current_input.read(cx).value().to_string();
        
        // Calculate all match positions for display
        if !search_term.is_empty() {
            let search_lower = search_term.to_lowercase();
            let content_lower = content.to_lowercase();
            
            self.search_positions.clear();
            for (pos, _) in content_lower.match_indices(&search_lower) {
                self.search_positions.push((pos, pos + search_term.len()));
            }
        } else {
            self.search_positions.clear();
        }
        
        let theme = cx.theme();
        
        v_flex()
            .p_4()
            .gap_3()
            .size_full()
            .child(
                TabBar::new("properties_tabs")
                    .selected_index(selected_index)
                    .underline()
                    .children(file_types.iter().map(|file_type| {
                        Tab::new().label(file_type.label())
                    }))
                    .on_click(cx.listener(move |this, index, _, _cx| {
                        if let Some(&file_type) = file_types.get(*index) {
                            this.current_file = file_type;
                            backend_handle.send(bridge::message::MessageToBackend::ReadServerFile {
                                name: server_name.as_str().into(),
                                filename: file_type.filename().into(),
                            });
                        }
                    }))
            )
            .child(
                h_flex()
                    .w_full()
                    .gap_2()
                    .items_center()
                    .child(
                        Input::new(&self.search_input)
                            .flex_1()
                    )
            )
            .child(
                v_flex()
                    .flex_1()
                    .border_1()
                    .border_color(theme.border)
                    .overflow_hidden()
                    .child(
                        if self.search_focused || !self.search_positions.is_empty() {
                            // Read-only view with highlighting when searching - styled like editable view
                            v_flex()
                                .size_full()
                                .overflow_y_scrollbar()
                                .p_2()
                                .bg(theme.highlight_theme.style.editor_background.unwrap_or(gpui::rgb(0x1e1e1e).into()))
                                .text_sm()
                                .line_height(gpui::relative(1.5))
                                .font_family("Monospace")
                                .child({
                                    let mut result = Vec::new();
                                    let lines: Vec<&str> = content.split('\n').collect();
                                    let mut byte_offset = 0;
                                    
                                    for line in lines {
                                        let line_start = byte_offset;
                                        let line_end = byte_offset + line.len();
                                        
                                        let mut spans = Vec::new();
                                        let mut pos = 0;
                                        
                                        for (start, end) in &self.search_positions {
                                            if *end <= line_start || *start >= line_end {
                                                continue;
                                            }
                                            
                                            let match_start = if *start > line_start { *start - line_start } else { 0 };
                                            let match_end = std::cmp::min(*end - line_start, line.len());
                                            
                                            if match_start > pos {
                                                spans.push(div()
                                                    .text_sm()
                                                    .line_height(gpui::relative(1.5))
                                                    .child(line[pos..match_start].to_string())
                                                    .into_any_element()
                                                );
                                            }
                                            
                                            spans.push(div()
                                                .bg(gpui::rgb(0x808080))
                                                .text_sm()
                                                .line_height(gpui::relative(1.5))
                                                .child(line[match_start..match_end].to_string())
                                                .into_any_element()
                                            );
                                            
                                            pos = match_end;
                                        }
                                        
                                        if pos < line.len() {
                                            spans.push(div()
                                                .text_sm()
                                                .line_height(gpui::relative(1.5))
                                                .child(line[pos..].to_string())
                                                .into_any_element()
                                            );
                                        }
                                        
                                        if spans.is_empty() {
                                            spans.push(div()
                                                .text_sm()
                                                .line_height(gpui::relative(1.5))
                                                .child(line.to_string())
                                                .into_any_element()
                                            );
                                        }
                                        
                                        result.push(
                                            h_flex()
                                                .w_full()
                                                .children(spans)
                                                .into_any_element()
                                        );
                                        
                                        byte_offset = line_end + 1;
                                    }
                                    
                                    v_flex().children(result)
                                })
                                .into_any_element()
                        } else {
                            // Editable view when not searching
                            Input::new(&current_input)
                                .size_full()
                                .p_2()
                                .bg(theme.highlight_theme.style.editor_background.unwrap_or(gpui::rgb(0x1e1e1e).into()))
                                .text_sm()
                                .line_height(gpui::relative(1.5))
                                .font_family("Monospace")
                                .border_0()
                                .into_any_element()
                        }
                    )
            )
    }
}
