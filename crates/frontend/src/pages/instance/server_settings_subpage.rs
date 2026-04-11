use gpui::{prelude::*, *};
use gpui_component::{v_flex, h_flex, input::{Input, InputState}};

pub struct ServerSettingsSubpage {
    pub server_name: SharedString,
    memory_min_input: Entity<InputState>,
    memory_max_input: Entity<InputState>,
    jvm_flags_input: Entity<InputState>,
}

impl ServerSettingsSubpage {
    pub fn new(server_name: SharedString, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let memory_min_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("512")
                .placeholder("Min memory in MB")
        });
        
        let memory_max_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("1024")
                .placeholder("Max memory in MB")
        });
        
        let jvm_flags_input = cx.new(|cx| {
            InputState::new(window, cx)
                .auto_grow(1, 4)
                .default_value("-XX:+UseG1GC -XX:MaxGCPauseMillis=200")
                .placeholder("JVM flags")
        });
        
        Self {
            server_name,
            memory_min_input,
            memory_max_input,
            jvm_flags_input,
        }
    }
}

impl Render for ServerSettingsSubpage {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .p_4()
            .gap_4()
            .child(div().text_lg().font_weight(gpui::FontWeight::BOLD).child("Server Settings"))
            .child(
                v_flex()
                    .gap_4()
                    .w_full()
                    // Memory Configuration
                    .child(
                        div()
                            .p_3()
                            .rounded_lg()
                            .border_1()
                            .child(
                                v_flex()
                                    .gap_3()
                                    .w_full()
                                    .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD).child("Memory Settings"))
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .w_full()
                                            .child(
                                                v_flex()
                                                    .flex_1()
                                                    .gap_1()
                                                    .child(div().text_xs().child("Minimum (MB)"))
                                                    .child(Input::new(&self.memory_min_input).max_w_64())
                                            )
                                            .child(
                                                v_flex()
                                                    .flex_1()
                                                    .gap_1()
                                                    .child(div().text_xs().child("Maximum (MB)"))
                                                    .child(Input::new(&self.memory_max_input).max_w_64())
                                            )
                                    )
                                    .child(div().text_xs().text_color(gpui::rgb(0x888888)).child("Adjust based on expected player count"))
                            )
                    )
                    // JVM Flags
                    .child(
                        div()
                            .p_3()
                            .rounded_lg()
                            .border_1()
                            .child(
                                v_flex()
                                    .gap_3()
                                    .w_full()
                                    .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD).child("JVM Flags"))
                                    .child(
                                        v_flex()
                                            .w_full()
                                            .gap_1()
                                            .child(Input::new(&self.jvm_flags_input).w_full())
                                    )
                                    .child(div().text_xs().text_color(gpui::rgb(0x888888)).child("Recommended: -XX:+UseG1GC -XX:MaxGCPauseMillis=200"))
                            )
                    )
                    // Info Section
                    .child(
                        div()
                            .p_3()
                            .rounded_lg()
                            .border_1()
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD).child("Performance Tips"))
                                    .child(div().text_xs().child("⚡ G1GC: Better for servers with many players"))
                                    .child(div().text_xs().child("⚡ Parallel GC: Good for CPU-heavy operations"))
                                    .child(div().text_xs().child("⚡ Start with defaults and adjust based on performance"))
                            )
                    )
            )
    }
}
