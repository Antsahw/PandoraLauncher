use bridge::handle::BackendHandle;
use bridge::message::MessageToBackend;
use crate::entity::DataEntities;
use gpui::{prelude::*, *};
use gpui_component::{v_flex, input::{Input, InputState, InputEvent}, select::{Select, SelectState, SelectEvent, SearchableVec}, IndexPath};
use std::path::PathBuf;
use serde_json;

pub struct ServerSettingsSubpage {
    pub server_name: SharedString,
    backend_handle: BackendHandle,
    data: DataEntities,
    java_runtime_select: Entity<SelectState<SearchableVec<&'static str>>>,
    
    // Memory settings
    memory_enabled: bool,
    memory_min_input_state: Entity<InputState>,
    memory_max_input_state: Entity<InputState>,
    
    // Start.sh script
    start_script_input_state: Entity<InputState>,
}

impl ServerSettingsSubpage {
    pub fn new(server_name: SharedString, backend_handle: BackendHandle, data: DataEntities, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Load the current configuration from the server config
        let saved_runtime = Self::load_server_java_runtime(&server_name);
        let (memory_min, memory_max, memory_enabled) = Self::load_server_memory(&server_name);
        
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
        
        let memory_min_input_state = cx.new(|cx| {
            InputState::new(window, cx).default_value(memory_min.to_string())
        });
        cx.subscribe(&memory_min_input_state, Self::on_memory_min_changed).detach();
        
        let memory_max_input_state = cx.new(|cx| {
            InputState::new(window, cx).default_value(memory_max.to_string())
        });
        cx.subscribe(&memory_max_input_state, Self::on_memory_max_changed).detach();
        
        let start_script_input_state = cx.new(|cx| {
            InputState::new(window, cx).auto_grow(1, 8).default_value("#!/bin/bash\n# Start script will be loaded from server\n")
        });
        cx.subscribe(&start_script_input_state, Self::on_start_script_changed).detach();
        
        let page = Self {
            server_name,
            backend_handle,
            data,
            java_runtime_select,
            memory_enabled,
            memory_min_input_state,
            memory_max_input_state,
            start_script_input_state,
        };
        
        // Subscribe to select changes
        cx.subscribe(&page.java_runtime_select, Self::on_java_runtime_selected).detach();
        
        // Request start.sh content from backend
        page.backend_handle.send(MessageToBackend::ReadServerFile {
            name: page.server_name.as_str().into(),
            filename: "start.sh".into(),
        });
        
        page
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
        
        self.backend_handle.send(MessageToBackend::SetServerJavaRuntime {
            name: self.server_name.as_str().into(),
            java_runtime: (*value).into(),
        });
    }
    
    fn on_memory_min_changed(
        &mut self,
        _: Entity<InputState>,
        event: &InputEvent,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::Change = event {
            if !self.memory_enabled {
                return;
            }
            let min = self.memory_min_input_state.read(cx).value().parse::<u32>().unwrap_or(512);
            let max = self.memory_max_input_state.read(cx).value().parse::<u32>().unwrap_or(1024);
            
            self.backend_handle.send(MessageToBackend::SetServerMemory {
                name: self.server_name.as_str().into(),
                min_memory: min,
                max_memory: max,
            });
        }
    }
    
    fn on_memory_max_changed(
        &mut self,
        _: Entity<InputState>,
        event: &InputEvent,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::Change = event {
            if !self.memory_enabled {
                return;
            }
            let min = self.memory_min_input_state.read(cx).value().parse::<u32>().unwrap_or(512);
            let max = self.memory_max_input_state.read(cx).value().parse::<u32>().unwrap_or(1024);
            
            self.backend_handle.send(MessageToBackend::SetServerMemory {
                name: self.server_name.as_str().into(),
                min_memory: min,
                max_memory: max,
            });
        }
    }
    
    
    fn on_start_script_changed(
        &mut self,
        _: Entity<InputState>,
        event: &InputEvent,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::Change = event {
            let script_content = self.start_script_input_state.read(cx).value();
            
            self.backend_handle.send(MessageToBackend::WriteServerFile {
                name: self.server_name.as_str().into(),
                filename: "start.sh".into(),
                content: script_content.to_string().into(),
            });
        }
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
    
    /// Load the saved memory settings for this server from its config file
    fn load_server_memory(server_name: &str) -> (u32, u32, bool) {
        let default_min = 512;
        let default_max = 1024;
        
        // Try PANDORA_DIR environment variable first
        if let Ok(pandora_dir) = std::env::var("PANDORA_DIR") {
            let config_path = PathBuf::from(pandora_dir)
                .join("servers")
                .join(server_name)
                .join(".minecraft/server_config.json");
            
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(java_obj) = json.get("java") {
                        let enabled = java_obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                        if let Some(memory_obj) = java_obj.get("memory") {
                            let min = memory_obj.get("min").and_then(|v| v.as_u64()).unwrap_or(default_min as u64) as u32;
                            let max = memory_obj.get("max").and_then(|v| v.as_u64()).unwrap_or(default_max as u64) as u32;
                            return (min, max, enabled);
                        }
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
                    if let Some(java_obj) = json.get("java") {
                        let enabled = java_obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                        if let Some(memory_obj) = java_obj.get("memory") {
                            let min = memory_obj.get("min").and_then(|v| v.as_u64()).unwrap_or(default_min as u64) as u32;
                            let max = memory_obj.get("max").and_then(|v| v.as_u64()).unwrap_or(default_max as u64) as u32;
                            return (min, max, enabled);
                        }
                    }
                }
            }
        }
        
        (default_min, default_max, false)
    }
}

impl Render for ServerSettingsSubpage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Load start.sh content from cache if available  
        let cached_start_script = {
            let files = self.data.server_files.read();
            files.get("start.sh").cloned()
        };
        
        if let Some(content) = cached_start_script {
            let current_value = self.start_script_input_state.read(cx).value();
            // Only update if it's still the default placeholder
            if current_value.contains("will be loaded") {
                self.start_script_input_state.update(cx, |state, input_cx| {
                    state.set_value(content.to_string(), window, input_cx);
                });
            }
        }
        
        v_flex()
            .p_4()
            .gap_4()
            .child(div().text_lg().font_weight(gpui::FontWeight::BOLD).child("Server Settings"))
            .child(
                v_flex()
                    .gap_4()
                    .w_full()
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
                            )
                    )
                    // Start.sh Script
                    .child(
                        div()
                            .p_3()
                            .rounded_lg()
                            .border_1()
                            .child(
                                v_flex()
                                    .gap_3()
                                    .w_full()
                                    .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD).child("Start Script (start.sh)"))
                                    .child(
                                        v_flex()
                                            .w_full()
                                            .child(
                                                Input::new(&self.start_script_input_state)
                                            )
                                    )
                            )
                    )
            )
    }
}
