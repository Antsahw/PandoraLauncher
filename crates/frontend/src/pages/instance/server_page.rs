use bridge::handle::BackendHandle;
use gpui::{prelude::*, *};
use gpui_component::{
    WindowExt, button::{Button, ButtonVariants}, h_flex, input::{Input, InputState}, tab::{Tab, TabBar}, v_flex
};
use serde::{Deserialize, Serialize};

use crate::{
    entity::DataEntities, icon::PandoraIcon, interface_config::InterfaceConfig, pages::{instance::server_settings_subpage::ServerSettingsSubpage, page::Page}, root, ts
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
        let subpage = subpage_type.create(server_name.clone(), data.backend_handle.clone(), window, cx);

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
                Button::new("kill_server")
                    .danger()
                    .icon(PandoraIcon::Close)
                    .label(ts!("instance.kill_instance"))
                    .on_click({
                        let backend_handle = backend_handle.clone();
                        let server_name = server_name.clone();
                        move |_, _, _| {
                            let _ = backend_handle.send(bridge::message::MessageToBackend::StopServer {
                                name: server_name.as_str().into(),
                            });
                        }
                    }),
            )
            .child(
                Button::new("rename_server")
                    .label("Rename")
                    .on_click({
                        let server_name = server_name.clone();
                        move |_, window, cx| {
                            window.open_dialog(cx, move |dialog, _, _| {
                                dialog
                                    .title("Rename Server")
                                    .overlay_closable(false)
                                    .flex()
                                    .line_height(rems(1.2))
                                    .child("Enter new server name:")
                            })
                        }
                    }),
            )
            .child(
                Button::new("delete_server")
                    .danger()
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
            self.subpage = subpage_type.create(self.server_name.clone(), self.backend_handle.clone(), window, cx);
        }

        let selected_index = match &self.subpage {
            ServerSubpage::Logs(_) => 0,
            ServerSubpage::Settings(_) => 1,
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
                    .on_click(cx.listener(|_, index, _, cx| {
                        let page_type = match *index {
                            0 => ServerSubpageType::Logs,
                            1 => ServerSubpageType::Settings,
                            _ => return,
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
}

impl ServerSubpageType {
    pub fn create(
        self,
        server_name: SharedString,
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
                    ServerSettingsSubpage::new(server_name.clone(), window, cx)
                }))
            }
        }
    }
}

#[derive(Clone)]
pub enum ServerSubpage {
    Logs(Entity<ServerLogsSubpage>),
    Settings(Entity<ServerSettingsSubpage>),
}

impl ServerSubpage {
    pub fn page_type(&self) -> ServerSubpageType {
        match self {
            ServerSubpage::Logs(_) => ServerSubpageType::Logs,
            ServerSubpage::Settings(_) => ServerSubpageType::Settings,
        }
    }

    pub fn into_any_element(self) -> AnyElement {
        match self {
            Self::Logs(entity) => entity.into_any_element(),
            Self::Settings(entity) => entity.into_any_element(),
        }
    }
}

// Server Logs Subpage
pub struct ServerLogsSubpage {
    server_name: SharedString,
    backend_handle: BackendHandle,
    command_input_state: Entity<InputState>,
}

impl ServerLogsSubpage {
    pub fn new(server_name: SharedString, backend_handle: BackendHandle, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let command_input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Type a command...")
        });
        
        Self {
            server_name,
            backend_handle,
            command_input_state,
        }
    }
}

impl Render for ServerLogsSubpage {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .p_4()
            .gap_4()
            .size_full()
            .child(
                div().child(format!("Server logs for: {}", self.server_name))
            )
            .child(
                div()
                    .child("Server logs will appear here when the server is running...")
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Input::new(&self.command_input_state)
                            .flex_1()
                    )
                    .child(
                        Button::new("send_command")
                            .label(ts!("common.send"))
                            .on_click({
                                let server_name = self.server_name.clone();
                                let backend_handle = self.backend_handle.clone();
                                let command_input = self.command_input_state.clone();
                                move |_, window, cx| {
                                    let command = command_input.read(cx).value();
                                    if !command.trim().is_empty() {
                                        backend_handle.send(bridge::message::MessageToBackend::SendServerCommand {
                                            name: server_name.as_str().into(),
                                            command: command.as_str().into(),
                                        });
                                        command_input.update(cx, |state, cx| state.set_value("", window, cx));
                                    }
                                }
                            })
                    )
            )
    }
}
