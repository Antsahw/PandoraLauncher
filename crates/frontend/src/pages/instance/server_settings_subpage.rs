use bridge::handle::BackendHandle;
use gpui::{prelude::*, *};
use gpui_component::{v_flex, h_flex, input::{Input, InputState}, select::{Select, SelectState, SelectEvent, SearchableVec}, IndexPath};
use std::path::PathBuf;
use serde_json;

pub struct ServerSettingsSubpage {
    pub server_name: SharedString,
    backend_handle: BackendHandle,
    memory_min_input: Entity<InputState>,
    memory_max_input: Entity<InputState>,
    jvm_flags_input: Entity<InputState>,
    java_runtime_select: Entity<SelectState<SearchableVec<&'static str>>>,
}

impl ServerSettingsSubpage {
    pub fn new(server_name: SharedString, backend_handle: BackendHandle, window: &mut Window, cx: &mut Context<Self>) -> Self {
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
        
        // Load the current java_runtime from the server config
        let saved_runtime = Self::load_server_java_runtime(&server_name);
        
        let java_runtime_select = cx.new(|cx| {
            let versions = SearchableVec::new(vec!["system", "java8", "java17", "java21", "java25"]);
            let mut select_state = SelectState::new(versions, None, window, cx).searchable(true);
            // Initialize with the saved runtime value
            if !saved_runtime.is_empty() {
                let versions_arr = ["system", "java8", "java17", "java21", "java25"];
                if let Some(index) = versions_arr.iter().position(|&v| v == saved_runtime.as_str()) {
                    select_state.set_selected_index(Some(IndexPath::new(index)), window, cx);
                }
            }
            select_state
        });
        cx.subscribe(&java_runtime_select, Self::on_java_runtime_selected).detach();
        
        Self {
            server_name,
            backend_handle,
            memory_min_input,
            memory_max_input,
            jvm_flags_input,
            java_runtime_select,
        }
    }
    
    fn on_java_runtime_selected(
        &mut self,
        _state: Entity<SelectState<SearchableVec<&'static str>>>,
        event: &SelectEvent<SearchableVec<&'static str>>,
        _cx: &mut Context<Self>,
    ) {
        let SelectEvent::Confirm(value) = event;
        let Some(value) = value else {
            return;
        };
        
        self.backend_handle.send(bridge::message::MessageToBackend::SetServerJavaRuntime {
            name: self.server_name.as_str().into(),
            java_runtime: (*value).into(),
        });
    }
    
    /// Load the saved java_runtime for this server from its config file
    fn load_server_java_runtime(server_name: &str) -> String {
        // Try PANDORA_DIR environment variable first
        if let Ok(pandora_dir) = std::env::var("PANDORA_DIR") {
            let config_path = PathBuf::from(pandora_dir)
                .join("servers")
                .join(server_name)
                .join(".minecraft/server_config.json");
            
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(runtime_name) = json.get("java")
                        .and_then(|java_obj| java_obj.get("runtime_name"))
                        .and_then(|v| v.as_str()) {
                        return runtime_name.to_string();
                    }
                }
            }
        }
        
        // Fall back to ~/.local/share/PandoraLauncher
        if let Ok(home_dir) = std::env::var("HOME") {
            let config_path = PathBuf::from(home_dir)
                .join(".local/share/PandoraLauncher/servers")
                .join(server_name)
                .join(".minecraft/server_config.json");
                
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(runtime_name) = json.get("java")
                        .and_then(|java_obj| java_obj.get("runtime_name"))
                        .and_then(|v| v.as_str()) {
                        return runtime_name.to_string();
                    }
                }
            }
        }
        
        String::new()
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
                    // Java Runtime Selection
                    .child(
                        div()
                            .p_3()
                            .rounded_lg()
                            .border_1()
                            .child(
                                v_flex()
                                    .gap_3()
                                    .w_full()
                                    .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD).child("Java Runtime"))
                                    .child(
                                        v_flex()
                                            .w_full()
                                            .gap_1()
                                            .child(Select::new(&self.java_runtime_select).max_w_64())
                                    )
                                    .child(div().text_xs().text_color(gpui::rgb(0x888888)).child("Select Java version: system, Java 8, 17, 21, or 25"))
                            )
                    )
            )
    }
}
