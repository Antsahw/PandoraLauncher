use std::sync::Arc;

use bridge::{handle::BackendHandle, message::{EmbeddedOrRaw, MessageToBackend}, modal_action::ModalAction};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, Selectable, WindowExt, alert::Alert, button::Button, checkbox::Checkbox, dialog::Dialog, h_flex, input::{Input, InputEvent, InputState}, select::{Select, SelectState}, skeleton::Skeleton, v_flex
};
use schema::version_manifest::{MinecraftVersionManifest, MinecraftVersionType};

use crate::{entity::{metadata::{AsMetadataResult, FrontendMetadata, FrontendMetadataResult, FrontendMetadataState}}, icon::PandoraIcon, interface_config::InterfaceConfig, pages::instances_page::VersionList, ts};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ServerLoader {
    Vanilla,
    Paper,
    Purpur,
    Fabric,
    Forge,
    NeoForge,
}

struct CreateServerModalState {
    metadata: Entity<FrontendMetadata>,
    versions: Entity<FrontendMetadataState>,
    backend_handle: BackendHandle,
    minecraft_version_dropdown: Entity<SelectState<VersionList>>,
    name_input_state: Entity<InputState>,
    selected_loader: ServerLoader,
    loaded_versions: bool,
    error_loading_versions: Option<SharedString>,
    name_invalid: bool,
    server_names: Arc<[SharedString]>,
    original_fallback_name: SharedString,
    unique_fallback_name: SharedString,
    icon: Option<EmbeddedOrRaw>,
    available_versions_for_software: Vec<String>,
    loading_software_versions: bool,
    server_software_versions_metadata: Option<Entity<FrontendMetadataState>>,
    _versions_updated_subscription: Subscription,
    _name_input_subscription: Subscription,
    _version_selected_subscription: Subscription,
    _software_versions_subscription: Option<Subscription>,
}

impl CreateServerModalState {
    pub fn new(metadata: Entity<FrontendMetadata>, backend_handle: BackendHandle, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let server_names: Arc<[SharedString]> = Arc::new([]);

        let minecraft_version_dropdown =
            cx.new(|cx| SelectState::new(VersionList::default(), None, window, cx).searchable(true));

        let _version_selected_subscription = cx.observe_in(&minecraft_version_dropdown, window, |this, _, window, cx| {
            this.update_fallback_name(window, cx);
        });

        let name_input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Unnamed Server")
        });

        let _name_input_subscription = {
            let server_names = Arc::clone(&server_names);
            cx.subscribe_in(&name_input_state, window, move |this, input_state, _: &InputEvent, _, cx| {
                let text = input_state.read(cx).value();

                if !text.as_str().is_empty() {
                    if !crate::is_valid_instance_name(text.as_str()) {
                        this.name_invalid = true;
                        return;
                    }
                }

                this.name_invalid = server_names.contains(&text);
            })
        };

        let versions = FrontendMetadata::request(&metadata, bridge::meta::MetadataRequest::MinecraftVersionManifest, cx);

        let _versions_updated_subscription = cx.observe_in(&versions, window, move |this, _, window, cx| {
            this.reload_version_dropdown(window, cx);
        });

        let mut this = Self {
            metadata,
            versions,
            backend_handle,
            minecraft_version_dropdown,
            name_input_state,
            selected_loader: ServerLoader::Vanilla,
            loaded_versions: false,
            error_loading_versions: None,
            name_invalid: false,
            server_names,
            original_fallback_name: Default::default(),
            unique_fallback_name: Default::default(),
            icon: None,
            available_versions_for_software: Vec::new(),
            loading_software_versions: false,
            server_software_versions_metadata: None,
            _versions_updated_subscription,
            _name_input_subscription,
            _version_selected_subscription,
            _software_versions_subscription: None,
        };

        this.reload_version_dropdown(window, cx);

        this
    }


    pub fn update_fallback_name(&mut self, window: &mut Window, cx: &mut App) {
        let selected = self.minecraft_version_dropdown
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or("Unnamed Server".into());

        if self.original_fallback_name != selected {
            self.original_fallback_name = selected.clone();

            if self.server_names.contains(&selected) {
                for i in 1..10 {
                    let new_name = SharedString::from(format!("{}-{}", selected, i));
                    if !self.server_names.contains(&new_name) {
                        self.unique_fallback_name = new_name.clone();
                        cx.update_entity(&self.name_input_state, |input_state, cx| {
                            input_state.set_placeholder(new_name, window, cx);
                        });
                        return;
                    }
                }
            }

            self.unique_fallback_name = selected.clone();
            cx.update_entity(&self.name_input_state, |input_state, cx| {
                input_state.set_placeholder(selected, window, cx);
            });
        }
    }

    pub fn reload_version_dropdown(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Get result and manifest data first, outside the closure
        let result: FrontendMetadataResult<MinecraftVersionManifest> = self.versions.read(cx).result();
        let (versions, latest) = match result {
            FrontendMetadataResult::Loading => {
                self.loaded_versions = false;
                self.error_loading_versions = None;
                (Vec::new(), None)
            },
            FrontendMetadataResult::Error(error) => {
                self.loaded_versions = false;
                self.error_loading_versions = Some(error);
                (Vec::new(), None)
            },
            FrontendMetadataResult::Loaded(manifest) => {
                self.loaded_versions = true;
                self.error_loading_versions = None;

                let show_snapshots = InterfaceConfig::get(cx).show_snapshots_in_create_instance;
                let mut versions: Vec<SharedString> = if show_snapshots {
                    manifest.versions.iter().map(|v| SharedString::from(v.id.as_str())).collect()
                } else {
                    manifest
                        .versions
                        .iter()
                        .filter(|v| !matches!(v.r#type, MinecraftVersionType::Snapshot))
                        .map(|v| SharedString::from(v.id.as_str()))
                        .collect()
                };

                // Filter by available versions for the current software if we have them
                if !self.available_versions_for_software.is_empty() {
                    versions.retain(|v| {
                        self.available_versions_for_software.iter()
                            .any(|av| av == v.as_str())
                    });
                }

                (versions, Some(SharedString::from(manifest.latest.release.as_str())))
            },
        };

        // Now update the dropdown with the filtered versions
        cx.update_entity(&self.minecraft_version_dropdown, |dropdown, cx| {
            let mut to_select = None;

            if let Some(last_selected) = dropdown.selected_value().cloned()
                && versions.contains(&last_selected)
            {
                to_select = Some(last_selected);
            }

            if to_select.is_none()
                && let Some(latest) = latest
                && versions.contains(&latest)
            {
                to_select = Some(latest);
            }

            if to_select.is_none() {
                to_select = versions.first().cloned();
            }

            dropdown.set_items(
                VersionList {
                    versions: versions.clone(),
                    matched_versions: versions,
                },
                window,
                cx,
            );

            if let Some(to_select) = to_select {
                dropdown.set_selected_value(&to_select, window, cx);
            }

            cx.notify();
        });

        if self.loaded_versions {
            self.update_fallback_name(window, cx);
        }
    }

    pub fn request_software_versions(&mut self, software: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.loading_software_versions = true;
        let software_str = software.to_string();
        
        // Request available versions from the backend
        let metadata_entity = FrontendMetadata::request(
            &self.metadata,
            bridge::meta::MetadataRequest::ServerSoftwareVersions(software_str.clone()),
            cx
        );
        
        self.server_software_versions_metadata = Some(metadata_entity.clone());
        
        // Set up subscription to handle response when it arrives
        let subscription = cx.observe_in(&metadata_entity, window, |this, _, window, cx| {
            if let Some(ref software_meta) = this.server_software_versions_metadata {
                let metadata_result: FrontendMetadataResult<Arc<schema::server_software_versions::ServerSoftwareVersions>> = 
                    software_meta.read(cx).result();
                
                match metadata_result {
                    FrontendMetadataResult::Loaded(software_versions) => {
                        this.available_versions_for_software = software_versions.versions.clone();
                        this.loading_software_versions = false;
                        this.reload_version_dropdown(window, cx);
                    },
                    FrontendMetadataResult::Error(_err) => {
                        this.available_versions_for_software.clear();
                        this.loading_software_versions = false;
                    },
                    FrontendMetadataResult::Loading => {
                        // Still loading
                    },
                }
            }
        });
        
        self._software_versions_subscription = Some(subscription);
    }

    pub fn render(&mut self, modal: Dialog, _window: &mut Window, cx: &mut Context<Self>) -> Dialog {
        // Check if we have a pending software versions request to process
        if let Some(ref software_meta) = self.server_software_versions_metadata {
            let metadata_result: FrontendMetadataResult<Arc<schema::server_software_versions::ServerSoftwareVersions>> = 
                software_meta.read(cx).result();
            
            if let FrontendMetadataResult::Loaded(software_versions) = metadata_result {
                if self.available_versions_for_software.is_empty() || self.available_versions_for_software != software_versions.versions {
                    self.available_versions_for_software = software_versions.versions.clone();
                    self.loading_software_versions = false;
                    // Re-apply filtering to show only available versions
                    self.reload_version_dropdown(_window, cx);
                }
            }
        }

        if let Some(error) = self.error_loading_versions.clone() {
            let error_widget = Alert::new("error", format!("{}", error))
                .icon(PandoraIcon::CircleX)
                .title(ts!("instance.versions_loading.error"));

            let metadata = self.metadata.clone();
            let reload_button =
                Button::new("reload-versions")
                    .label(ts!("instance.versions_loading.reload"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.error_loading_versions = None;
                        FrontendMetadata::force_reload(&metadata, bridge::meta::MetadataRequest::MinecraftVersionManifest, cx);
                    }));

            return modal
                .title("Create Server")
                .child(v_flex().gap_3().child(error_widget).child(reload_button))
                .footer(Button::new("ok").label(ts!("common.ok")).on_click(|_, window, cx| window.close_dialog(cx)));
        }

        let version_dropdown;
        let show_snapshots_button;
        let loader_button_group;

        if !self.loaded_versions {
            version_dropdown = Select::new(&self.minecraft_version_dropdown)
                .w_full()
                .placeholder(ts!("instance.versions_loading.game_versions"))
                .into_any_element();
            show_snapshots_button = Skeleton::new().w_full().min_h_4().max_h_4().rounded_md().into_any_element();
            loader_button_group = Skeleton::new().w_full().min_h_8().max_h_8().rounded_md().into_any_element();
        } else {
            version_dropdown = Select::new(&self.minecraft_version_dropdown).title_prefix(format!("{}: ", ts!("instance.mc_version"))).into_any_element();
            show_snapshots_button = Checkbox::new("show_snapshots")
                .checked(InterfaceConfig::get(cx).show_snapshots_in_create_instance)
                .label(ts!("instance.show_snapshots"))
                .on_click(cx.listener(move |this, show, window, cx| {
                    InterfaceConfig::get_mut(cx).show_snapshots_in_create_instance = *show;
                    this.reload_version_dropdown(window, cx);
                }))
                .into_any_element();
            loader_button_group = v_flex()
                .gap_2()
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("loader-vanilla")
                                .label("Vanilla")
                                .selected(self.selected_loader == ServerLoader::Vanilla)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.selected_loader = ServerLoader::Vanilla;
                                    this.request_software_versions("Vanilla", window, cx);
                                }))
                                .flex_1()
                        )
                        .child(
                            Button::new("loader-paper")
                                .label("Paper")
                                .selected(self.selected_loader == ServerLoader::Paper)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.selected_loader = ServerLoader::Paper;
                                    this.request_software_versions("Paper", window, cx);
                                }))
                                .flex_1()
                        )
                        .child(
                            Button::new("loader-purpur")
                                .label("Purpur")
                                .selected(self.selected_loader == ServerLoader::Purpur)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.selected_loader = ServerLoader::Purpur;
                                    this.request_software_versions("Purpur", window, cx);
                                }))
                                .flex_1()
                        )
                        .child(
                            Button::new("loader-fabric")
                                .label("Fabric")
                                .selected(self.selected_loader == ServerLoader::Fabric)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.selected_loader = ServerLoader::Fabric;
                                    this.request_software_versions("Fabric", window, cx);
                                }))
                                .flex_1()
                        )
                )
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("loader-forge")
                                .label("Forge")
                                .selected(self.selected_loader == ServerLoader::Forge)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.selected_loader = ServerLoader::Forge;
                                    this.request_software_versions("Forge", window, cx);
                                }))
                                .flex_1()
                        )
                        .child(
                            Button::new("loader-neoforge")
                                .label("NeoForge")
                                .selected(self.selected_loader == ServerLoader::NeoForge)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.selected_loader = ServerLoader::NeoForge;
                                    this.request_software_versions("NeoForge", window, cx);
                                }))
                                .flex_1()
                        )

                )
                .into_any_element();
        };

        let content = v_flex()
            .gap_3()
            .child(crate::labelled(
                ts!("instance.name"),
                Input::new(&self.name_input_state).when(self.name_invalid, |this| this.border_color(cx.theme().danger)),
            ))
            .child(crate::labelled(ts!("instance.version"), v_flex().gap_2().child(version_dropdown).child(show_snapshots_button)))
            .child(crate::labelled("Server Software", v_flex().gap_2()
                .child(loader_button_group)
                .child(div().text_xs().text_color(gpui::rgb(0x888888)).child("Note: Some versions may not be available for all server software types."))
            ))
            .child(h_flex().child(Button::new("icon").icon(PandoraIcon::Plus).label(ts!("instance.select_icon")).on_click({
                let entity = cx.entity();
                move |_, window, cx| {
                    let entity = entity.clone();
                    crate::modals::select_icon::open_select_icon(Box::new(move |icon, cx| {
                        cx.update_entity(&entity, |this, _| {
                            this.icon = Some(icon);
                        });
                    }), window, cx);
                }
            })));

        let name_is_invalid = self.name_invalid;
        modal
            .overlay_closable(false)
            .title("Create Server")
            .child(content)
            .footer(
                h_flex()
                    .gap_2()
                    .w_full()
                    .child(Button::new("cancel").flex_1().label(ts!("common.cancel"))
                        .on_click(|_, window, cx| window.close_dialog(cx)))
                    .child(Button::new("ok").flex_1().label(ts!("common.ok"))
                        .on_click(cx.listener(move |this, _, window: &mut Window, cx| {
                            if name_is_invalid || !this.loaded_versions {
                                return;
                            }
                            let Some(selected_version) = this.minecraft_version_dropdown.read(cx).selected_value().cloned() else {
                                return;
                            };

                            let mut name = this.name_input_state.read(cx).value().clone();
                            if name.is_empty() {
                                name = this.unique_fallback_name.clone();
                            }

                            let server_software = match this.selected_loader {
                                ServerLoader::Vanilla => "Vanilla",
                                ServerLoader::Paper => "Paper",
                                ServerLoader::Purpur => "Purpur",
                                ServerLoader::Fabric => "Fabric",
                                ServerLoader::Forge => "Forge",
                                ServerLoader::NeoForge => "NeoForge",
                            };

                            let modal_action = ModalAction::default();
                            this.backend_handle.send(MessageToBackend::CreateServer {
                                name: name.as_str().into(),
                                version: selected_version.as_str().into(),
                                server_software: server_software.into(),
                                icon: this.icon.clone(),
                                modal_action: modal_action.clone(),
                            });
                            
                            // Show progress modal for server installation
                            crate::modals::generic::show_notification(window, cx, ts!("server.creating"), modal_action);
                            window.close_dialog(cx);
                        })))
            )
    }
}

pub fn open_create_server(
    metadata: Entity<FrontendMetadata>,
    backend_handle: BackendHandle,
    window: &mut Window,
    cx: &mut App,
) {
    let state = cx.new(|cx| {
        CreateServerModalState::new(metadata, backend_handle, window, cx)
    });

    window.open_dialog(cx, move |modal, window, cx| {
        cx.update_entity(&state, |state, cx| {
            state.render(modal, window, cx)
        })
    });
}
