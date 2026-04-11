# InstanceSettingsSubpage - Complete Architecture Analysis

This document provides a thorough analysis of how InstanceSettingsSubpage loads, manages, and persists instance settings. Use this as a reference for implementing ServerSettingsSubpage.

---

## 1. DATA STRUCTURES

### Located in: `crates/schema/src/instance.rs`

#### InstanceConfiguration (Parent)
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstanceConfiguration {
    pub minecraft_version: Ustr,
    pub loader: Loader,
    #[serde(default, skip_serializing_if = "crate::skip_if_none")]
    pub preferred_loader_version: Option<Ustr>,
    #[serde(default, deserialize_with = "crate::try_deserialize", skip_serializing_if = "crate::skip_if_none")]
    pub preferred_account: Option<Uuid>,
    #[serde(default, deserialize_with = "crate::try_deserialize", skip_serializing_if = "is_default_memory_configuration")]
    pub memory: Option<InstanceMemoryConfiguration>,
    #[serde(default, deserialize_with = "crate::try_deserialize", skip_serializing_if = "is_default_wrapper_command_configuration")]
    pub wrapper_command: Option<InstanceWrapperCommandConfiguration>,
    #[serde(default, deserialize_with = "crate::try_deserialize", skip_serializing_if = "is_default_jvm_flags_configuration")]
    pub jvm_flags: Option<InstanceJvmFlagsConfiguration>,
    #[serde(default, deserialize_with = "crate::try_deserialize", skip_serializing_if = "is_default_jvm_binary_configuration")]
    pub jvm_binary: Option<InstanceJvmBinaryConfiguration>,
    #[serde(default, deserialize_with = "crate::try_deserialize", skip_serializing_if = "is_default_linux_wrapper_configuration")]
    pub linux_wrapper: Option<InstanceLinuxWrapperConfiguration>,
    #[serde(default, deserialize_with = "crate::try_deserialize", skip_serializing_if = "is_default_system_libraries_configuration")]
    pub system_libraries: Option<InstanceSystemLibrariesConfiguration>,
    #[serde(default, deserialize_with = "crate::try_deserialize", skip_serializing_if = "crate::skip_if_none")]
    pub instance_fallback_icon: Option<Ustr>,
    #[serde(default, deserialize_with = "crate::try_deserialize")]
    pub disable_file_syncing: bool,
}
```

#### Configurable Settings Types

**InstanceMemoryConfiguration** - JVM Memory Management
```rust
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct InstanceMemoryConfiguration {
    pub enabled: bool,
    pub min: u32,      // in MiB
    pub max: u32,      // in MiB
}

impl InstanceMemoryConfiguration {
    pub const DEFAULT_MIN: u32 = 512;   // 512 MiB minimum
    pub const DEFAULT_MAX: u32 = 4096;  // 4 GB maximum
}

impl Default for InstanceMemoryConfiguration {
    fn default() -> Self {
        Self {
            enabled: false,
            min: Self::DEFAULT_MIN,
            max: Self::DEFAULT_MAX
        }
    }
}
```

**InstanceWrapperCommandConfiguration** - Wrapper Command (e.g., for GameMode)
```rust
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct InstanceWrapperCommandConfiguration {
    pub enabled: bool,
    pub flags: Arc<str>,  // Shell command/flags to prepend to launch
}
```

**InstanceJvmFlagsConfiguration** - JVM Arguments
```rust
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct InstanceJvmFlagsConfiguration {
    pub enabled: bool,
    pub flags: Arc<str>,  // JVM flags like: -XX:+UseG1GC -Xmx4G
}
```

**InstanceJvmBinaryConfiguration** - Custom Java Binary
```rust
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct InstanceJvmBinaryConfiguration {
    pub enabled: bool,
    pub path: Option<Arc<Path>>,  // Path to custom java/javaw executable
}
```

**InstanceLinuxWrapperConfiguration** - Linux-Specific Tools
```rust
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct InstanceLinuxWrapperConfiguration {
    #[serde(default, deserialize_with = "crate::try_deserialize")]
    pub use_mangohud: bool,  // MangoHUD overlay
    #[serde(default, deserialize_with = "crate::try_deserialize")]
    pub use_gamemode: bool,  // GameMode for performance
    #[serde(default = "crate::default_true", deserialize_with = "crate::try_deserialize")]
    pub use_discrete_gpu: bool,  // Use dedicated GPU (default: true)
    #[serde(default, deserialize_with = "crate::try_deserialize")]
    pub disable_gl_threaded_optimizations: bool,
}

impl Default for InstanceLinuxWrapperConfiguration {
    fn default() -> Self {
        Self {
            use_mangohud: false,
            use_gamemode: false,
            use_discrete_gpu: true,
            disable_gl_threaded_optimizations: false
        }
    }
}
```

**InstanceSystemLibrariesConfiguration** - LWJGL Libraries (OpenGL/Audio)
```rust
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct InstanceSystemLibrariesConfiguration {
    pub override_glfw: bool,        // Override GLFW (windowing)
    pub glfw: LwjglLibraryPath,     // GLFW library path
    pub override_openal: bool,      // Override OpenAL (audio)
    pub openal: LwjglLibraryPath,   // OpenAL library path
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub enum LwjglLibraryPath {
    #[default]
    Auto,                           // Use system default
    AutoPreferred(Arc<Path>),       // Use if exists, fallback to auto
    Explicit(Arc<Path>),            // Explicit path (must exist)
}

impl LwjglLibraryPath {
    pub fn get_or_auto(self, auto: &Option<Arc<Path>>) -> Option<Arc<Path>> {
        match self {
            LwjglLibraryPath::Auto => auto.clone(),
            LwjglLibraryPath::AutoPreferred(preferred) => {
                if preferred.exists() { Some(preferred) } else { auto.clone() }
            },
            LwjglLibraryPath::Explicit(path) => Some(path),
        }
    }
}
```

---

## 2. FRONTEND STATE STRUCTURE

### Located in: `crates/frontend/src/pages/instance/settings_subpage.rs`

#### InstanceSettingsSubpage Struct
```rust
pub struct InstanceSettingsSubpage {
    // Core references
    data: DataEntities,                                    // Shared app data
    instance: Entity<InstanceEntry>,                       // Instance being edited
    instance_id: InstanceID,
    backend_handle: BackendHandle,                         // Communication to backend

    // Basic settings UI state
    new_name_input_state: Entity<InputState>,              // Instance name editor
    new_name_change_state: NewNameChangeState,

    // Version management
    version_state: TypelessFrontendMetadataResult,         // Loading state for versions
    version_select_state: Entity<SelectState<VersionList>>,

    account_items: Entity<SelectState<NamedDropdown<Uuid>>>,

    // Loader selection
    loader: Loader,                                         // Current loader (Vanilla/Fabric/Forge/NeoForge)
    loader_select_state: Entity<SelectState<Vec<&'static str>>>,
    loader_versions_state: TypelessFrontendMetadataResult,  // Loading state
    loader_version_select_state: Entity<SelectState<SearchableVec<&'static str>>>,

    disable_file_syncing: bool,

    // Memory settings
    memory_override_enabled: bool,
    memory_min_input_state: Entity<InputState>,            // Min RAM (MiB)
    memory_max_input_state: Entity<InputState>,            // Max RAM (MiB)

    // JVM configuration
    wrapper_command_enabled: bool,
    wrapper_command_input_state: Entity<InputState>,
    jvm_flags_enabled: bool,
    jvm_flags_input_state: Entity<InputState>,
    jvm_binary_enabled: bool,
    jvm_binary_path: Option<PathLabel>,

    // System library overrides
    override_glfw_enabled: bool,
    override_glfw_path: Option<PathLabel>,
    override_openal_enabled: bool,
    override_openal_path: Option<PathLabel>,

    // Linux-specific options
    #[cfg(target_os = "linux")]
    use_mangohud: bool,
    #[cfg(target_os = "linux")]
    use_gamemode: bool,
    #[cfg(target_os = "linux")]
    use_discrete_gpu: bool,
    #[cfg(target_os = "linux")]
    disable_gl_threaded_optimizations: bool,
    #[cfg(target_os = "linux")]
    mangohud_available: bool,                              // Check: command -v mangohud
    #[cfg(target_os = "linux")]
    gamemode_available: bool,                              // Check: command -v gamemoderun

    // Instance root folder
    instance_root_label: PathLabel,

    // Subscriptions (keep alive for reactive updates)
    _observe_loader_version_subscription: Option<Subscription>,
    _select_file_task: Task<()>,
}

#[derive(PartialEq, Eq)]
enum NewNameChangeState {
    NoChange,
    InvalidName,
    Pending,  // Waiting for backend confirmation
}
```

---

## 3. INITIALIZATION (LOADING)

### Constructor: `InstanceSettingsSubpage::new()`

```rust
impl InstanceSettingsSubpage {
    pub fn new(
        instance: &Entity<InstanceEntry>,
        data: &DataEntities,
        backend_handle: BackendHandle,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> Self {
        // Step 1: Read current instance data
        let entry = instance.read(cx);
        let instance_id = entry.id;
        let instance_name = entry.name.clone();
        let loader = entry.configuration.loader;
        let preferred_loader_version = entry.configuration.preferred_loader_version
            .map(|s| s.as_str())
            .unwrap_or("Latest");
        let account = entry.configuration.preferred_account;
        let disable_file_syncing = entry.configuration.disable_file_syncing;

        // Extract settings with defaults
        let memory = entry.configuration.memory.unwrap_or_default();
        let wrapper_command = entry.configuration.wrapper_command.clone().unwrap_or_default();
        let jvm_flags = entry.configuration.jvm_flags.clone().unwrap_or_default();
        let jvm_binary = entry.configuration.jvm_binary.clone().unwrap_or_default();

        #[cfg(target_os = "linux")]
        let linux_wrapper = entry.configuration.linux_wrapper.unwrap_or_default();
        let system_libraries = entry.configuration.system_libraries.clone().unwrap_or_default();

        let instance_root_label = PathLabel::new(entry.root_path.clone(), true);

        // Process library paths (Auto vs Explicit)
        let glfw_path = system_libraries.glfw.get_or_auto(&*AUTO_LIBRARY_PATH_GLFW);
        let openal_path = system_libraries.openal.get_or_auto(&*AUTO_LIBRARY_PATH_OPENAL);

        // Step 2: Create UI state entities with initialization
        let new_name_input_state = cx.new(|cx| {
            InputState::new(window, cx).default_value(instance_name)
        });
        cx.subscribe(&new_name_input_state, Self::on_new_name_input).detach();

        // Step 3: Request Minecraft versions metadata (async)
        let minecraft_versions = FrontendMetadata::request(
            &data.metadata,
            MetadataRequest::MinecraftVersionManifest,
            cx
        );

        // Step 4: Create version selector
        let version_select_state = cx.new(|cx| {
            SelectState::new(VersionList::default(), None, window, cx).searchable(true)
        });
        // Subscribe to metadata updates
        cx.observe_in(&minecraft_versions, window, |page, versions, window, cx| {
            page.update_minecraft_versions(versions, window, cx);
        }).detach();
        cx.subscribe(&version_select_state, Self::on_minecraft_version_selected).detach();

        // Step 5: Setup account selector
        let account_items = cx.new(|cx| {
            let accounts = &data.accounts.read(cx).accounts;
            let mut account_items = Vec::with_capacity(accounts.len());
            let mut selected = None;

            for (index, loop_account) in accounts.iter().enumerate() {
                account_items.push(NamedDropdownItem {
                    name: loop_account.username.clone().into(),
                    item: loop_account.uuid,
                });
                if let Some(preferred_account) = account
                    && loop_account.uuid == preferred_account
                {
                    selected = Some(IndexPath::new(index));
                }
            }

            SelectState::new(NamedDropdown::new(account_items), selected, window, cx)
                .searchable(true)
        });
        cx.observe_in(&data.accounts, window, |page, accounts, window, cx| {
            page.update_account_list(accounts, window, cx);
        }).detach();

        // Step 6: Loader selector (static list)
        let loader_select_state = cx.new(|cx| {
            let loaders = Loader::iter()
                .filter(|l| *l != Loader::Unknown)
                .map(|l| l.name())
                .collect();
            let mut state = SelectState::new(loaders, None, window, cx);
            state.set_selected_value(&loader.name(), window, cx);
            state
        });
        cx.subscribe_in(&loader_select_state, window, Self::on_loader_selected).detach();

        // Step 7: Watch for instance changes to update dependent states
        cx.observe_in(instance, window, |page, instance, window, cx| {
            let entry = instance.read(cx);
            page.instance_root_label = PathLabel::new(entry.root_path.clone(), true);
            if page.loader_version_select_state.read(cx).selected_index(cx).is_none() {
                let version = entry.configuration.preferred_loader_version
                    .map(|s| s.as_str())
                    .unwrap_or("Latest");
                page.loader_version_select_state.update(cx, |select_state, cx| {
                    select_state.set_selected_value(&version, window, cx);
                });
            }
        }).detach();

        // Step 8: Loader version selector (dynamic based on loader choice)
        let loader_version_select_state = cx.new(|cx| {
            let mut select_state = SelectState::new(
                SearchableVec::new(vec![]),
                None,
                window,
                cx
            ).searchable(true);
            select_state.set_selected_value(&preferred_loader_version, window, cx);
            select_state
        });
        cx.subscribe(&loader_version_select_state, Self::on_loader_version_selected).detach();

        // Step 9: Memory input states
        let memory_min_input_state = cx.new(|cx| {
            InputState::new(window, cx).default_value(memory.min.to_string())
        });
        cx.subscribe_in(&memory_min_input_state, window, Self::on_memory_step).detach();
        cx.subscribe(&memory_min_input_state, Self::on_memory_changed).detach();

        let memory_max_input_state = cx.new(|cx| {
            InputState::new(window, cx).default_value(memory.max.to_string())
        });
        cx.subscribe_in(&memory_max_input_state, window, Self::on_memory_step).detach();
        cx.subscribe(&memory_max_input_state, Self::on_memory_changed).detach();

        // Step 10: Wrapper command input
        let wrapper_command_input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .auto_grow(1, 8)
                .default_value(wrapper_command.flags)
        });
        cx.subscribe(&wrapper_command_input_state, Self::on_wrapper_command_changed).detach();

        // Step 11: JVM flags input
        let jvm_flags_input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .auto_grow(1, 8)
                .default_value(jvm_flags.flags)
        });
        cx.subscribe(&jvm_flags_input_state, Self::on_jvm_flags_changed).detach();

        // Step 12: Construct the page with all state
        let mut page = Self {
            data: data.clone(),
            instance: instance.clone(),
            instance_id,
            new_name_input_state,
            version_state: TypelessFrontendMetadataResult::Loading,
            version_select_state,
            account_items,
            loader,
            loader_select_state,
            loader_version_select_state,
            disable_file_syncing,
            memory_override_enabled: memory.enabled,
            memory_min_input_state,
            memory_max_input_state,
            wrapper_command_enabled: wrapper_command.enabled,
            wrapper_command_input_state,
            jvm_flags_enabled: jvm_flags.enabled,
            jvm_flags_input_state,
            jvm_binary_enabled: jvm_binary.enabled,
            jvm_binary_path: jvm_binary.path.clone().map(|path| PathLabel::new(path, false)),
            override_glfw_enabled: system_libraries.override_glfw,
            override_glfw_path: glfw_path.map(|path| PathLabel::new(path, false)),
            override_openal_enabled: system_libraries.override_openal,
            override_openal_path: openal_path.map(|path| PathLabel::new(path, false)),
            instance_root_label,
            #[cfg(target_os = "linux")]
            use_mangohud: linux_wrapper.use_mangohud,
            #[cfg(target_os = "linux")]
            use_gamemode: linux_wrapper.use_gamemode,
            #[cfg(target_os = "linux")]
            use_discrete_gpu: linux_wrapper.use_discrete_gpu,
            #[cfg(target_os = "linux")]
            disable_gl_threaded_optimizations: linux_wrapper.disable_gl_threaded_optimizations,
            #[cfg(target_os = "linux")]
            mangohud_available: Self::is_command_available("mangohud"),
            #[cfg(target_os = "linux")]
            gamemode_available: Self::is_command_available("gamemoderun"),
            new_name_change_state: NewNameChangeState::NoChange,
            backend_handle,
            loader_versions_state: TypelessFrontendMetadataResult::Loading,
            _observe_loader_version_subscription: None,
            _select_file_task: Task::ready(())
        };

        // Step 13: Trigger initial data loading
        page.update_minecraft_versions(minecraft_versions, window, cx);
        page.update_loader_versions(window, cx);

        page
    }
}
```

---

## 4. DATA PERSISTENCE AND BACKEND MESSAGE FLOW

### Backend Messages

Located in: `crates/bridge/src/message.rs`

```rust
pub enum MessageToBackend {
    // Basic settings
    SetInstanceMinecraftVersion {
        id: InstanceID,
        version: Ustr
    },
    SetInstanceLoader {
        id: InstanceID,
        loader: Loader
    },
    SetInstancePreferredAccount {
        id: InstanceID,
        account: Option<Uuid>,
    },
    SetInstancePreferredLoaderVersion {
        id: InstanceID,
        loader_version: Option<&'static str>
    },
    SetInstanceDisableFileSyncing {
        id: InstanceID,
        disable_file_syncing: bool,
    },

    // Advanced settings
    SetInstanceMemory {
        id: InstanceID,
        memory: InstanceMemoryConfiguration,
    },
    SetInstanceWrapperCommand {
        id: InstanceID,
        wrapper_command: InstanceWrapperCommandConfiguration,
    },
    SetInstanceJvmFlags {
        id: InstanceID,
        jvm_flags: InstanceJvmFlagsConfiguration,
    },
    SetInstanceJvmBinary {
        id: InstanceID,
        jvm_binary: InstanceJvmBinaryConfiguration,
    },
    SetInstanceLinuxWrapper {
        id: InstanceID,
        linux_wrapper: InstanceLinuxWrapperConfiguration,
    },
    SetInstanceSystemLibraries {
        id: InstanceID,
        system_libraries: InstanceSystemLibrariesConfiguration,
    },
}
```

### Backend Handler Processing

Located in: `crates/backend/src/backend_handler.rs`

All message handlers follow the same pattern:

```rust
MessageToBackend::SetInstanceMemory { id, memory } => {
    if let Some(instance) = self.instance_state.write().instances.get_mut(id) {
        instance.configuration.modify(|configuration| {
            configuration.memory = Some(memory);
        });
    }
}
MessageToBackend::SetInstanceWrapperCommand { id, wrapper_command } => {
    if let Some(instance) = self.instance_state.write().instances.get_mut(id) {
        instance.configuration.modify(|configuration| {
            configuration.wrapper_command = Some(wrapper_command);
        });
    }
}
MessageToBackend::SetInstanceJvmFlags { id, jvm_flags } => {
    if let Some(instance) = self.instance_state.write().instances.get_mut(id) {
        instance.configuration.modify(|configuration| {
            configuration.jvm_flags = Some(jvm_flags);
        });
    }
}
MessageToBackend::SetInstanceJvmBinary { id, jvm_binary } => {
    if let Some(instance) = self.instance_state.write().instances.get_mut(id) {
        instance.configuration.modify(|configuration| {
            configuration.jvm_binary = Some(jvm_binary);
        });
    }
}
MessageToBackend::SetInstanceLinuxWrapper { id, linux_wrapper } => {
    if let Some(instance) = self.instance_state.write().instances.get_mut(id) {
        instance.configuration.modify(|configuration| {
            configuration.linux_wrapper = Some(linux_wrapper);
        });
    }
}
MessageToBackend::SetInstanceSystemLibraries { id, system_libraries } => {
    if let Some(instance) = self.instance_state.write().instances.get_mut(id) {
        instance.configuration.modify(|configuration| {
            configuration.system_libraries = Some(system_libraries);
        });
    }
}
```

### Persistence Layer

Located in: `crates/backend/src/persistent.rs`

```rust
pub struct Persistent<T: Serialize + for <'de> Deserialize<'de>> {
    path: Arc<Path>,
    dirty: bool,                    // External modification flag
    data: T
}

impl<T: Serialize + for <'de> Deserialize<'de>> Persistent<T> {
    /// Core persistence function - called by all backends
    pub fn modify(&mut self, func: impl FnOnce(&mut T)) {
        // Check for external modifications
        if self.dirty {
            self.load_from_disk();
        }

        // Apply the modification
        (func)(&mut self.data);

        // Serialize to JSON
        if let Ok(bytes) = serde_json::to_vec(&self.data) {
            // Safe atomic write (prevents corruption)
            if crate::write_safe(&self.path, &bytes).is_ok() {
                self.dirty = true;  // Mark as known to be written
            }
        }
    }

    pub fn get(&mut self) -> &T {
        // Reload if externally modified
        if self.dirty {
            self.load_from_disk();
        }
        &self.data
    }

    fn load_from_disk(&mut self) {
        self.dirty = false;
        let Ok(data) = crate::read_json(&self.path) else {
            return;  // Keep existing data on error
        };
        self.data = data;
    }
}
```

**Key Features**:
1. **Atomic writes**: Uses `write_safe()` to prevent corruption
2. **Dirty detection**: Tracks external file modifications
3. **Automatic reload**: Syncs with disk on next read if external change detected
4. **JSON serialization**: All configs stored as pretty JSON files

---

## 5. FRONTEND EVENT HANDLERS & SAVING

### Memory Settings Example

```rust
pub fn on_memory_changed(
    &mut self,
    _: Entity<InputState>,
    event: &InputEvent,
    cx: &mut Context<Self>,
) {
    if let InputEvent::Change = event {
        // Immediately read current UI state and send to backend
        self.backend_handle.send(MessageToBackend::SetInstanceMemory {
            id: self.instance_id,
            memory: self.get_memory_configuration(cx)
        });
    }
}

fn get_memory_configuration(&self, cx: &App) -> InstanceMemoryConfiguration {
    let min = self.memory_min_input_state.read(cx).value()
        .parse::<u32>()
        .unwrap_or(0);
    let max = self.memory_max_input_state.read(cx).value()
        .parse::<u32>()
        .unwrap_or(0);

    InstanceMemoryConfiguration {
        enabled: self.memory_override_enabled,
        min,
        max
    }
}
```

### Memory Step Button Handling (256 MB increments)

```rust
pub fn on_memory_step(
    &mut self,
    state: &Entity<InputState>,
    event: &NumberInputEvent,
    window: &mut Window,
    cx: &mut Context<Self>,
) {
    match event {
        NumberInputEvent::Step(step_action) => match step_action {
            gpui_component::input::StepAction::Decrement => {
                if let Ok(mut value) = state.read(cx).value().parse::<u32>() {
                    // Decrement: divide by 256, subtract 1, multiply by 256, min 128
                    value = value.saturating_div(256)
                        .saturating_sub(1)
                        .saturating_mul(256)
                        .max(128);
                    state.update(cx, |input, cx| {
                        input.set_value(value.to_string(), window, cx);
                    })
                }
            },
            gpui_component::input::StepAction::Increment => {
                if let Ok(mut value) = state.read(cx).value().parse::<u32>() {
                    // Increment: divide by 256, add 1, multiply by 256
                    value = value.saturating_div(256)
                        .saturating_add(1)
                        .saturating_mul(256)
                        .max(128);
                    state.update(cx, |input, cx| {
                        input.set_value(value.to_string(), window, cx);
                    })
                }
            },
        },
    }
}
```

### JVM Flags Example

```rust
pub fn on_jvm_flags_changed(
    &mut self,
    _: Entity<InputState>,
    event: &InputEvent,
    cx: &mut Context<Self>,
) {
    if let InputEvent::Change = event {
        self.backend_handle.send(MessageToBackend::SetInstanceJvmFlags {
            id: self.instance_id,
            jvm_flags: self.get_jvm_flags_configuration(cx)
        });
    }
}

fn get_jvm_flags_configuration(&self, cx: &App) -> InstanceJvmFlagsConfiguration {
    let flags = self.jvm_flags_input_state.read(cx).value();
    InstanceJvmFlagsConfiguration {
        enabled: self.jvm_flags_enabled,
        flags: flags.into(),
    }
}
```

### JVM Binary Selection (File Picker)

```rust
pub fn select_file(
    &mut self,
    message: SharedString,
    handle: impl FnOnce(&mut Self, Option<Arc<Path>>) + 'static,
    window: &mut Window,
    cx: &mut Context<Self>
) {
    let receiver = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some(message)
    });

    let this_entity = cx.entity();
    self._select_file_task = window.spawn(cx, async move |cx| {
        let Ok(result) = receiver.await else { return; };
        _ = cx.update_window_entity(&this_entity, move |this, window, cx| {
            match result {
                Ok(Some(paths)) => {
                    (handle)(this, paths.first().map(|v| v.as_path().into()));
                    cx.notify();
                },
                Ok(None) => {},
                Err(error) => {
                    let error = format!("{}", error);
                    let notification = Notification::new()
                        .autohide(false)
                        .with_type(NotificationType::Error)
                        .title(error);
                    window.push_notification(notification, cx);
                },
            }
        });
    });
}
```

Usage in render UI:
```rust
PathLabel::button_opt(&self.jvm_binary_path, "select_jvm_binary")
    .disabled(!jvm_binary_enabled)
    .on_click(cx.listener(|this, _, window, cx| {
        this.select_file(
            ts!("instance.select_jvm_binary"),
            |this, path| {
                this.jvm_binary_path = path.map(|path| PathLabel::new(path, false));
                this.backend_handle.send(MessageToBackend::SetInstanceJvmBinary {
                    id: this.instance_id,
                    jvm_binary: this.get_jvm_binary_configuration()
                });
            },
            window,
            cx
        );
    }))
```

---

## 6. COMPLEX STATE MANAGEMENT - LOADER VERSIONS

The loader version management showcases the pattern for handling dependent data:

```rust
fn update_loader_versions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let loader_versions = match self.loader {
        // Vanilla has no loader versions
        Loader::Vanilla | Loader::Unknown => {
            self._observe_loader_version_subscription = None;
            self.loader_versions_state = TypelessFrontendMetadataResult::Loaded;
            vec![""]
        },
        // Fabric: fetch from metadata
        Loader::Fabric => {
            self.update_loader_versions_for_loader(
                MetadataRequest::FabricLoaderManifest,
                |manifest: &FabricLoaderManifest| {
                    std::iter::once("Latest")
                        .chain(manifest.0.iter().map(|s| s.version.as_str()))
                        .collect()
                },
                window,
                cx
            )
        },
        Loader::Forge => {
            self.update_loader_versions_for_loader(
                MetadataRequest::ForgeMavenManifest,
                |manifest: &ForgeMavenManifest| {
                    std::iter::once("Latest")
                        .chain(manifest.0.iter().map(|s| s.as_str()))
                        .collect()
                },
                window,
                cx
            )
        },
        Loader::NeoForge => {
            self.update_loader_versions_for_loader(
                MetadataRequest::NeoforgeMavenManifest,
                |manifest: &NeoforgeMavenManifest| {
                    std::iter::once("Latest")
                        .chain(manifest.0.iter().map(|s| s.as_str()))
                        .collect()
                },
                window,
                cx
            )
        },
    };

    let preferred_loader_version = self.instance.read(cx)
        .configuration.preferred_loader_version
        .map(|s| s.as_str())
        .unwrap_or("Latest");

    // Update the select state with new versions and restore selection
    self.loader_version_select_state.update(cx, move |select_state, cx| {
        select_state.set_items(SearchableVec::new(loader_versions), window, cx);
        select_state.set_selected_value(&preferred_loader_version, window, cx);
    });
}

fn update_loader_versions_for_loader<T>(
    &mut self,
    request: MetadataRequest,
    items_fn: impl Fn(&T) -> Vec<&'static str> + 'static,
    window: &mut Window,
    cx: &mut Context<Self>
) -> Vec<&'static str>
where
    FrontendMetadataState: AsMetadataResult<T>,
{
    // Request metadata from backend
    let request = FrontendMetadata::request(&self.data.metadata, request, cx);

    // Get immediate result (may be loading)
    let result: FrontendMetadataResult<T> = request.read(cx).result();
    let items = match &result {
        FrontendMetadataResult::Loading => vec![],
        FrontendMetadataResult::Loaded(manifest) => (items_fn)(&manifest),
        FrontendMetadataResult::Error(_) => vec![],
    };

    self.loader_versions_state = result.as_typeless();

    // Subscribe to updates when metadata finishes loading
    self._observe_loader_version_subscription = Some(cx.observe_in(
        &request,
        window,
        move |page, metadata, window, cx| {
            let result: FrontendMetadataResult<T> = metadata.read(cx).result();
            let versions = if let FrontendMetadataResult::Loaded(manifest) = &result {
                (items_fn)(&manifest)
            } else {
                vec![]
            };
            page.loader_versions_state = result.as_typeless();

            let preferred_loader_version = page.instance.read(cx)
                .configuration.preferred_loader_version
                .map(|s| s.as_str())
                .unwrap_or("Latest");

            page.loader_version_select_state.update(cx, move |select_state, cx| {
                select_state.set_items(SearchableVec::new(versions), window, cx);
                select_state.set_selected_value(&preferred_loader_version, window, cx);
            });
        }
    ));

    items
}
```

---

## 7. UI RENDERING (3 SECTIONS)

Located in: `impl Render for InstanceSettingsSubpage`

### Section 1: Basic Settings
- Instance name editor with validation
- Minecraft version selector (with loading state)
- Mod loader selector
- Loader version selector (dynamic)
- Account override selector
- File syncing toggle

### Section 2: Runtime Settings
- Memory configuration (with step buttons)
- JVM flags input
- JVM binary selector
- GLFW library override
- OpenAL library override
- Wrapper command input
- Linux-specific tools (MangoHUD, GameMode, discrete GPU, GL threading)

### Section 3: Actions
- Instance folder relocation
- Create desktop shortcut
- Delete instance (with safety confirmation)

---

## 8. KEY PATTERNS FOR SERVER IMPLEMENTATION

### Pattern 1: Immediate Persistence
```rust
// When UI changes:
self.backend_handle.send(MessageToBackend::SetServerSetting {
    id: self.server_id,
    setting: value,
});
// Backend immediately: instance.configuration.modify(|c| c.setting = value);
// Persistence: automatic via Persistent<T>.modify()
```

### Pattern 2: Getter Functions for Configuration Objects
```rust
fn get_configuration(&self, cx: &App) -> ServerConfiguration {
    ServerConfiguration {
        enabled: self.enabled,
        value: self.input_state.read(cx).value(),
    }
}
```

### Pattern 3: Metadata-Driven Dropdowns
```rust
// 1. Request metadata in init:
let versions = FrontendMetadata::request(&data.metadata, MetadataRequest::..., cx);

// 2. Observe updates:
cx.observe_in(&versions, window, |page, meta, window, cx| {
    page.update_dropdown(meta, window, cx);
}).detach();

// 3. Handle selection:
cx.subscribe(&select_state, Self::on_selection).detach();
```

### Pattern 4: Conditional UI Based on Feature Availability
```rust
#[cfg(target_os = "linux")]
let linux_content = v_flex().child(...);

// In render:
#[cfg(target_os = "linux")]
let runtime_content = runtime_content.child(linux_content);
```

### Pattern 5: File Picker Pattern
```rust
PathLabel::button_opt(&self.path, "button_id")
    .on_click(cx.listener(|this, _, window, cx| {
        this.select_file(ts!("prompt_message"), |this, path| {
            this.path = path.map(|p| PathLabel::new(p, false));
            this.backend_handle.send(MessageToBackend::SetServerPath {
                id: this.server_id,
                path: this.path.as_ref().map(PathLabel::path),
            });
        }, window, cx);
    }))
```

---

## 9. DATA FLOW DIAGRAM

```
User Changes Setting in UI
        ↓
Event Handler Called (on_xxx_changed)
        ↓
Read UI state → Construct configuration object
        ↓
backend_handle.send(MessageToBackend::SetServer...)
        ↓
Backend receives message
        ↓
Find instance by ID in ServerState
        ↓
Call: instance.configuration.modify(|c| c.field = value)
        ↓
Persistent<T>.modify() executes:
  1. Check dirty flag (external changes)
  2. Apply modification closure
  3. Serialize to JSON
  4. Atomic write_safe() to disk
  5. Mark dirty=true
        ↓
Configuration persisted to disk ✓
        ↓
Frontend observes instance change
        ↓
UI re-renders with new values
```

---

## 10. IMPORTANT IMPLEMENTATION CONSIDERATIONS

1. **Immediate Save Pattern**: Every change is sent to backend immediately. No "Save" button needed.

2. **Validation**: Name validation happens in UI (`is_valid_instance_name()`), not backend.

3. **Defaults Handling**: 
   - Optional fields in schema
   - `unwrap_or_default()` when reading
   - Custom `is_default_*()` functions for serialization skip

4. **Subscriptions**: Entity subscriptions must be stored as fields to keep them alive

5. **Async Metadata**: Metadata requests return Entity<FrontendMetadataState>, not the data directly

6. **Safe File Writes**: Uses atomic write (`write_safe()`) to prevent corruption

7. **Dirty Flag**: Prevents losing external changes while app is running

8. **Memory Alignment**: Memory values aligned to 256 MB boundaries for cleanliness

9. **Linux Feature Gating**: Some options only available on Linux (compile-gated)

10. **Account Override**: `None` means no override, not "default account"

---

## Summary: Architecture Strengths

✓ **Immediate Persistence**: No unsaved changes - single source of truth on disk
✓ **Type Safe**: Schema types prevent invalid configurations
✓ **Reactive Updates**: Metadata and backend changes update UI automatically
✓ **Conflict Resolution**: Dirty flag handles external modifications
✓ **Cross-Platform**: Conditional compilation for OS-specific features
✓ **Scalable**: Easy to add new settings - just add schema field → UI input → backend message → handler

This architecture is production-grade and proven. Use it as the foundation for ServerSettingsSubpage.
