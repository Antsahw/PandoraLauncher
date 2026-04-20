use bridge::{
    handle::BackendHandle, instance::InstanceID, message::MessageToBackend
};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, button::Button, checkbox::Checkbox, h_flex, input::{Input, InputEvent, InputState}, v_flex, StyledExt
};
use schema::instance::InstanceSandboxConfiguration;
use ustr::Ustr;

use crate::ts;

pub struct InstanceSandboxSubpage {
    instance_id: InstanceID,
    backend_handle: BackendHandle,
    sandbox_enabled: bool,
    allowed_paths: Vec<Ustr>,
    new_path_input_state: Entity<InputState>,
}

impl InstanceSandboxSubpage {
    pub fn new(
        instance: &Entity<crate::entity::instance::InstanceEntry>,
        backend_handle: BackendHandle,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> Self {
        let entry = instance.read(cx);
        let instance_id = entry.id;
        let sandbox = entry.configuration.sandbox.clone().unwrap_or_default();

        let new_path_input_state = cx.new(|cx| {
            InputState::new(window, cx)
        });
        cx.subscribe(&new_path_input_state, Self::on_path_input).detach();

        Self {
            instance_id,
            backend_handle,
            sandbox_enabled: sandbox.enabled,
            allowed_paths: sandbox.allowed_paths,
            new_path_input_state,
        }
    }

    fn get_sandbox_configuration(&self) -> InstanceSandboxConfiguration {
        InstanceSandboxConfiguration {
            enabled: self.sandbox_enabled,
            allowed_paths: self.allowed_paths.clone(),
        }
    }

    pub fn on_path_input(
        &mut self,
        _: Entity<InputState>,
        _event: &InputEvent,
        _cx: &mut gpui::Context<Self>,
    ) {
        // Placeholder for path input handling
    }

    fn add_path(&mut self, cx: &mut gpui::Context<Self>) {
        let input_value = self.new_path_input_state.read(cx).value();
        
        if !input_value.is_empty() {
            let path = Ustr::from(input_value.trim());
            if !self.allowed_paths.contains(&path) {
                self.allowed_paths.push(path);
                self.backend_handle.send(MessageToBackend::SetInstanceSandboxConfiguration {
                    id: self.instance_id,
                    sandbox: self.get_sandbox_configuration(),
                });
                cx.notify();
            }
        }
    }

    fn remove_path(&mut self, index: usize, cx: &mut gpui::Context<Self>) {
        if index < self.allowed_paths.len() {
            self.allowed_paths.remove(index);
            self.backend_handle.send(MessageToBackend::SetInstanceSandboxConfiguration {
                id: self.instance_id,
                sandbox: self.get_sandbox_configuration(),
            });
            cx.notify();
        }
    }
}

impl Render for InstanceSandboxSubpage {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let sandbox_enabled = self.sandbox_enabled;
        let paths = self.allowed_paths.clone();

        v_flex()
            .gap_4()
            .p_4()
            .child(
                h_flex()
                    .gap_3()
                    .mb_4()
                    .child(div().text_lg().font_bold().child(ts!("instance.sandbox.title")))
            )
            .child(
                Checkbox::new("sandbox_enabled")
                    .label(ts!("instance.linux.use_sandbox"))
                    .checked(sandbox_enabled)
                    .on_click(cx.listener(|page, value, _, cx| {
                        page.sandbox_enabled = *value;
                        page.backend_handle.send(MessageToBackend::SetInstanceSandboxConfiguration {
                            id: page.instance_id,
                            sandbox: page.get_sandbox_configuration(),
                        });
                        cx.notify();
                    }))
            )
            .when(sandbox_enabled, |this| {
                this.child(
                    v_flex()
                        .gap_3()
                        .child(
                            div()
                                .text_sm()
                                .font_semibold()
                                .child(ts!("instance.sandbox.allowed_paths"))
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .children(paths.iter().enumerate().map(|(index, path)| {
                                    h_flex()
                                        .gap_2()
                                        .w_full()
                                        .child(
                                            div()
                                                .flex_grow()
                                                .p_2()
                                                .bg(theme.background)
                                                .border_1()
                                                .border_color(theme.border)
                                                .rounded(theme.radius)
                                                .child(div().text_xs().child(path.as_str()))
                                        )
                                        .child(
                                            Button::new(format!("remove_path_{}", index))
                                                .label("Remove")
                                                .on_click({
                                                    let idx = index;
                                                    cx.listener(move |page, _: &ClickEvent, _window, cx| {
                                                        page.remove_path(idx, cx);
                                                    })
                                                })
                                        )
                                }))
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .child(
                                    Input::new(&self.new_path_input_state)
                                )
                                .child(
                                    Button::new("add_path")
                                        .label(ts!("instance.sandbox.add_path"))
                                        .on_click(cx.listener(|page, _: &ClickEvent, _window, cx| {
                                            page.add_path(cx);
                                        }))
                                )
                        )
                )
            })
    }
}
