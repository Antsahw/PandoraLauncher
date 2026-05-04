# Instance Settings Structure - Complete Analysis

## Overview

Instance settings in Pandora Launcher follow a consistent end-to-end pattern:
1. **Schema Layer** (`crates/schema/src/instance.rs`) - Data structures and serialization
2. **Frontend Layer** (`crates/frontend/src/pages/instance/settings_subpage.rs`) - UI rendering and state management  
3. **Bridge Layer** (`crates/bridge/src/message.rs`) - Message definitions for communication
4. **Backend Layer** (`crates/backend/src/backend_handler.rs`) - Message processing and persistence

---

## 1. SCHEMA DEFINITIONS

**File**: [crates/schema/src/instance.rs](crates/schema/src/instance.rs)

### Parent Configuration Structure

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
    #[serde(default, deserialize_with = "crate::try_deserialize", skip_serializing_if = "is_default_java_runtime_configuration")]
    pub java_runtime: Option<InstanceJavaRuntimeConfiguration>,
    #[serde(default, deserialize_with = "crate::try_deserialize", skip_serializing_if = "is_default_linux_wrapper_configuration")]
    pub linux_wrapper: Option<InstanceLinuxWrapperConfiguration>,
    #[serde(default, deserialize_with = "crate::try_deserialize", skip_serializing_if = "is_default_system_libraries_configuration")]
    pub system_libraries: Option<InstanceSystemLibrariesConfiguration>,
    #[serde(default, deserialize_with = "crate::try_deserialize", skip_serializing_if = "crate::skip_if_none")]
    pub instance_fallback_icon: Option<Ustr>,
    #[serde(default, deserialize_with = "crate::try_deserialize")]
    pub disable_file_syncing: bool,
    #[serde(default, deserialize_with = "crate::try_deserialize")]
    pub skip_integrity_check: bool,  // ← TOGGLE SETTING
}
```

### Individual Setting Structures

**InstanceMemoryConfiguration** - JVM Memory Override
```rust
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct InstanceMemoryConfiguration {
    pub enabled: bool,
    pub min: u32,     // MiB
    pub max: u32,     // MiB
}
```

**InstanceWrapperCommandConfiguration** - Wrapper Command (e.g., GameMode)
```rust
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct InstanceWrapperCommandConfiguration {
    pub enabled: bool,
    pub flags: Arc<str>,  // Shell command to prepend to launch
}
```

**InstanceJvmFlagsConfiguration** - JVM Arguments
```rust
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct InstanceJvmFlagsConfiguration {
    pub enabled: bool,
    pub flags: Arc<str>,  // e.g., "-XX:+UseG1GC"
}
```

**InstanceJvmBinaryConfiguration** - Custom Java Binary
```rust
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct InstanceJvmBinaryConfiguration {
    pub enabled: bool,
    pub path: Option<Arc<Path>>,
    pub forced_java_version: Option<u32>,  // 8, 11, 17, 21, etc.
}
```

**InstanceJavaRuntimeConfiguration** - Select Java Runtime
```rust
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct InstanceJavaRuntimeConfiguration {
    pub enabled: bool,
    pub runtime_name: String,  // "system", "java8", "java17", "java21"
}
```

**InstanceLinuxWrapperConfiguration** - Linux-Specific Tools
```rust
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct InstanceLinuxWrapperConfiguration {
    pub use_mangohud: bool,
    pub use_gamemode: bool,
    pub use_discrete_gpu: bool,
    pub disable_gl_threaded_optimizations: bool,
}
```

**InstanceSystemLibrariesConfiguration** - LWJGL Library Overrides
```rust
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct InstanceSystemLibrariesConfiguration {
    pub override_glfw: bool,    // Override GLFW (windowing)
    pub glfw: LwjglLibraryPath,
    pub override_openal: bool,  // Override OpenAL (audio)
    pub openal: LwjglLibraryPath,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub enum LwjglLibraryPath {
    #[default]
    Auto,                      // Use system default
    AutoPreferred(Arc<Path>),  // Use if exists, else fallback to auto
    Explicit(Arc<Path>),       // Explicit path (must exist)
}
```

---

## 2. FRONTEND UI IMPLEMENTATION

**File**: [crates/frontend/src/pages/instance/settings_subpage.rs](crates/frontend/src/pages/instance/settings_subpage.rs)

### Frontend State Structure

The `InstanceSettingsSubpage` struct manages all UI state:

```rust
pub struct InstanceSettingsSubpage {
    // Core references
    data: DataEntities,
    instance: Entity<InstanceEntry>,
    instance_id: InstanceID,
    backend_handle: BackendHandle,

    // Basic settings
    new_name_input_state: Entity<InputState>,
    disable_file_syncing: bool,
    skip_integrity_check: bool,  // ← FRONTEND STATE

    // Version & loader selection
    version_state: TypelessFrontendMetadataResult,
    version_select_state: Entity<SelectState<VersionList>>,
    loader: Loader,
    loader_select_state: Entity<SelectState<Vec<&'static str>>>,
    loader_versions_state: TypelessFrontendMetadataResult,
    loader_version_select_state: Entity<SelectState<SearchableVec<&'static str>>>,

    // Memory settings
    memory_override_enabled: bool,
    memory_min_input_state: Entity<InputState>,
    memory_max_input_state: Entity<InputState>,

    // JVM configuration
    wrapper_command_enabled: bool,
    wrapper_command_input_state: Entity<InputState>,
    jvm_flags_enabled: bool,
    jvm_flags_input_state: Entity<InputState>,
    jvm_binary_enabled: bool,
    jvm_binary_path: Option<PathLabel>,
    
    // Java runtime selection
    java_runtime_enabled: bool,
    java_runtime_select: Entity<SelectState<SearchableVec<&'static str>>>,

    // Linux-specific
    #[cfg(target_os = "linux")]
    use_mangohud: bool,
    #[cfg(target_os = "linux")]
    use_gamemode: bool,
    #[cfg(target_os = "linux")]
    use_discrete_gpu: bool,
    #[cfg(target_os = "linux")]
    disable_gl_threaded_optimizations: bool,

    // System library overrides
    override_glfw_enabled: bool,
    override_glfw_path: Option<PathLabel>,
    override_openal_enabled: bool,
    override_openal_path: Option<PathLabel>,
}
```

### Initialization Pattern

**Lines 83-95**: Load configuration from instance
```rust
let entry = instance.read(cx);
let instance_id = entry.id;
let instance_name = entry.name.clone();
let loader = entry.configuration.loader;
let preferred_loader_version = entry.configuration.preferred_loader_version
    .map(|s| s.as_str())
    .unwrap_or("Latest");
let account = entry.configuration.preferred_account;
let disable_file_syncing = entry.configuration.disable_file_syncing;
let skip_integrity_check = entry.configuration.skip_integrity_check;
```

**Lines 97-103**: Extract settings with defaults
```rust
let memory = entry.configuration.memory.unwrap_or_default();
let wrapper_command = entry.configuration.wrapper_command.clone().unwrap_or_default();
let jvm_flags = entry.configuration.jvm_flags.clone().unwrap_or_default();
let jvm_binary = entry.configuration.jvm_binary.clone().unwrap_or_default();
let java_runtime = entry.configuration.java_runtime.clone().unwrap_or_default();
let linux_wrapper = entry.configuration.linux_wrapper.clone().unwrap_or_default();
let system_libraries = entry.configuration.system_libraries.clone().unwrap_or_default();
```

### Example: Memory Override UI Rendering

**Lines 831-851**: Memory UI section
```rust
.child(v_flex()
    .gap_1()
    .child(Checkbox::new("memory")
        .label(ts!("instance.memory"))
        .checked(memory_override_enabled)
        .on_click(cx.listener(|page, value, _, cx| {
            if page.memory_override_enabled != *value {
                page.memory_override_enabled = *value;
                page.backend_handle.send(MessageToBackend::SetInstanceMemory {
                    id: page.instance_id,
                    memory: page.get_memory_configuration(cx)
                });
                cx.notify();
            }
        }))
    )
    .child(h_flex()
        .gap_1()
        .child(v_flex()
            .w_full()
            .gap_1()
            .child(NumberInput::new(&self.memory_min_input_state).small().suffix("MiB").disabled(!memory_override_enabled))
            .child(NumberInput::new(&self.memory_max_input_state).small().suffix("MiB").disabled(!memory_override_enabled))
        )
        .child(v_flex()
            .gap_1()
            .child(ts!("common.min"))
            .child(ts!("common.max"))
        )
    )
)
```

### Example: Integrity Check Toggle UI

**Lines 816-821**: Integrity check toggle
```rust
.child(crate::labelled(
    "Integrity Check",
    Checkbox::new("integrity")
        .label("Skip integrity check")
        .checked(self.skip_integrity_check)
        .on_click(cx.listener(|page, value, _, _| {
            page.skip_integrity_check = *value;
            page.backend_handle.send(MessageToBackend::SetInstanceSkipIntegrityCheck {
                id: page.instance_id,
                skip_integrity_check: *value
            });
        }))
))
```

### Helper Methods

**Get Configuration Methods**: Convert UI state to schema objects
```rust
fn get_memory_configuration(&self, cx: &App) -> InstanceMemoryConfiguration {
    let min = self.memory_min_input_state.read(cx).value().parse::<u32>().unwrap_or(0);
    let max = self.memory_max_input_state.read(cx).value().parse::<u32>().unwrap_or(0);
    InstanceMemoryConfiguration {
        enabled: self.memory_override_enabled,
        min,
        max
    }
}

fn get_jvm_flags_configuration(&self, cx: &App) -> InstanceJvmFlagsConfiguration {
    let flags = self.jvm_flags_input_state.read(cx).value();
    InstanceJvmFlagsConfiguration {
        enabled: self.jvm_flags_enabled,
        flags: flags.into(),
    }
}

fn get_java_runtime_configuration(&self, cx: &mut gpui::Context<Self>) -> InstanceJavaRuntimeConfiguration {
    let runtime_name = self.java_runtime_select.read(cx).selected_value()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "system".to_string());
    InstanceJavaRuntimeConfiguration {
        enabled: self.java_runtime_enabled,
        runtime_name,
    }
}
```

---

## 3. BRIDGE MESSAGE DEFINITIONS

**File**: [crates/bridge/src/message.rs](crates/bridge/src/message.rs)

### MessageToBackend Enum

Instance-related message variants (lines 84-128):

```rust
pub enum MessageToBackend {
    // ... other messages ...

    // Instance configuration messages
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
    SetInstanceJavaRuntime {
        id: InstanceID,
        java_runtime: InstanceJavaRuntimeConfiguration,
    },
    SetInstanceLinuxWrapper {
        id: InstanceID,
        linux_wrapper: InstanceLinuxWrapperConfiguration,
    },
    SetInstanceSystemLibraries {
        id: InstanceID,
        system_libraries: InstanceSystemLibrariesConfiguration,
    },

    // ... other messages ...
}
```

### ⚠️ MISSING MESSAGE

The frontend is trying to send `SetInstanceSkipIntegrityCheck`, but this message **IS NOT defined** in the enum:

```rust
// Frontend line 819:
page.backend_handle.send(MessageToBackend::SetInstanceSkipIntegrityCheck {
    id: page.instance_id,
    skip_integrity_check: *value
});

// ❌ This variant doesn't exist in message.rs!
```

---

## 4. BACKEND MESSAGE HANDLING

**File**: [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs)

### Message Handler Pattern

All SetInstance* messages follow the same pattern (lines 333-390):

```rust
pub async fn handle_message(self: &Arc<Self>, message: MessageToBackend) {
    match message {
        MessageToBackend::SetInstanceDisableFileSyncing { id, disable_file_syncing } => {
            if let Some(instance) = self.instance_state.write().instances.get_mut(id) {
                instance.configuration.modify(|configuration| {
                    configuration.disable_file_syncing = disable_file_syncing;
                });
            }
            self.apply_syncing_to_instance(id);
        },

        MessageToBackend::SetInstanceMemory { id, memory } => {
            if let Some(instance) = self.instance_state.write().instances.get_mut(id) {
                instance.configuration.modify(|configuration| {
                    configuration.memory = Some(memory);
                });
            }
        },

        MessageToBackend::SetInstanceWrapperCommand { id, wrapper_command } => {
            if let Some(instance) = self.instance_state.write().instances.get_mut(id) {
                instance.configuration.modify(|configuration| {
                    configuration.wrapper_command = Some(wrapper_command);
                });
            }
        },

        MessageToBackend::SetInstanceJvmFlags { id, jvm_flags } => {
            if let Some(instance) = self.instance_state.write().instances.get_mut(id) {
                instance.configuration.modify(|configuration| {
                    configuration.jvm_flags = Some(jvm_flags);
                });
            }
        },

        MessageToBackend::SetInstanceJvmBinary { id, jvm_binary } => {
            if let Some(instance) = self.instance_state.write().instances.get_mut(id) {
                instance.configuration.modify(|configuration| {
                    configuration.jvm_binary = Some(jvm_binary);
                });
            }
        },

        MessageToBackend::SetInstanceJavaRuntime { id, java_runtime } => {
            if let Some(instance) = self.instance_state.write().instances.get_mut(id) {
                instance.configuration.modify(|configuration| {
                    configuration.java_runtime = Some(java_runtime);
                });
            }
        },

        MessageToBackend::SetInstanceLinuxWrapper { id, linux_wrapper } => {
            if let Some(instance) = self.instance_state.write().instances.get_mut(id) {
                instance.configuration.modify(|configuration| {
                    configuration.linux_wrapper = Some(linux_wrapper);
                });
            }
        },

        MessageToBackend::SetInstanceSystemLibraries { id, system_libraries } => {
            if let Some(instance) = self.instance_state.write().instances.get_mut(id) {
                instance.configuration.modify(|configuration| {
                    configuration.system_libraries = Some(system_libraries);
                });
            }
        },

        // ... other messages ...
    }
}
```

### Key Pattern: `Persistent<T>` with `.modify()`

The configuration is stored as `Persistent<InstanceConfiguration>` which handles serialization:
- `.modify(|config| { ... })` - Automatically persists changes to `info_v1.json`
- No manual file I/O needed
- Uses Serde for serialization

---

## 5. COMPLETE END-TO-END FLOW EXAMPLE

### Example: Memory Override Setting

#### 1️⃣ **Schema Layer** (Define)
- [crates/schema/src/instance.rs](crates/schema/src/instance.rs#L62)
- `InstanceConfiguration` contains `Option<InstanceMemoryConfiguration>`
- Serialized to `info_v1.json`

#### 2️⃣ **Frontend Layer** (Display & Collect)
- [crates/frontend/src/pages/instance/settings_subpage.rs](crates/frontend/src/pages/instance/settings_subpage.rs#L94)
- Load: `entry.configuration.memory.unwrap_or_default()`
- Store locally: `memory_override_enabled: bool`, `memory_min_input_state`, `memory_max_input_state`
- Render: `NumberInput` components for min/max RAM
- Handler: `on_memory_changed()` → calls `get_memory_configuration()`

#### 3️⃣ **Bridge Layer** (Transport)
- [crates/bridge/src/message.rs](crates/bridge/src/message.rs#L104)
- Frontend sends: `MessageToBackend::SetInstanceMemory { id, memory }`
- Message crossed network (IPC or socket)

#### 4️⃣ **Backend Layer** (Persist)
- [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs#L341)
- Backend receives message
- Updates: `instance.configuration.modify(|c| { c.memory = Some(memory); })`
- Automatically persisted to `info_v1.json`

#### 5️⃣ **Verification**
- File saved at: `~/.pandora/instances/{instance_id}/info_v1.json`
- When instance re-opens, memory settings are restored from JSON

---

## 6. CONFIGURATION FILES ON DISK

Instance configuration is stored as JSON:

```
~/.pandora/instances/{instance_id}/
├── info_v1.json          ← Stored InstanceConfiguration
├── worlds/
├── server-logs/
├── .minecraft/
│   ├── mods/
│   ├── resourcepacks/
│   └── ...
└── ...
```

Example `info_v1.json`:
```json
{
  "minecraft_version": "1.20.1",
  "loader": "Fabric",
  "preferred_loader_version": "0.14.25",
  "preferred_account": "550e8400-e29b-41d4-a716-446655440000",
  "memory": {
    "enabled": true,
    "min": 1024,
    "max": 8192
  },
  "jvm_flags": {
    "enabled": true,
    "flags": "-XX:+UseG1GC -XX:+ParallelRefProcEnabled"
  },
  "disable_file_syncing": false,
  "skip_integrity_check": false
}
```

---

## 7. SUMMARY OF KEY FILES & RESPONSIBILITIES

| File | Responsibility | Key Components |
|------|-----------------|-----------------|
| [crates/schema/src/instance.rs](crates/schema/src/instance.rs) | Define configuration structures | `InstanceConfiguration`, all `Instance*Configuration` structs |
| [crates/frontend/src/pages/instance/settings_subpage.rs](crates/frontend/src/pages/instance/settings_subpage.rs) | UI rendering & user interaction | `InstanceSettingsSubpage` struct, event handlers, helper methods |
| [crates/bridge/src/message.rs](crates/bridge/src/message.rs) | Frontend-backend communication | `MessageToBackend` enum with all `SetInstance*` variants |
| [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs) | Process messages & persist | `handle_message()` match arms for each message type |
| `info_v1.json` | Persistent storage | Serialized `InstanceConfiguration` |

---

## 8. ADDING A NEW SETTING - CHECKLIST

To add a new instance setting (e.g., a new toggle):

- [ ] **1. Schema** - Add field to `InstanceConfiguration` in [crates/schema/src/instance.rs](crates/schema/src/instance.rs)
- [ ] **2. Frontend State** - Add field to `InstanceSettingsSubpage` struct in [crates/frontend/src/pages/instance/settings_subpage.rs](crates/frontend/src/pages/instance/settings_subpage.rs)
- [ ] **3. Frontend Init** - Initialize field in `new()` method from `entry.configuration`
- [ ] **4. Frontend Render** - Add UI component (Checkbox, Input, etc.) in `render()` method
- [ ] **5. Frontend Handler** - Add event handler and helper method (`get_*_configuration()`)
- [ ] **6. Bridge Message** - Add variant to `MessageToBackend` enum in [crates/bridge/src/message.rs](crates/bridge/src/message.rs)
- [ ] **7. Backend Handler** - Add match arm in `handle_message()` in [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs)

---

## 9. ISSUES FOUND

### ⚠️ Missing SetInstanceSkipIntegrityCheck Message

**Location**: Frontend line 819 attempts to send undefined message

**Error**: `MessageToBackend::SetInstanceSkipIntegrityCheck` variant not found

**Current Status**:
- ✅ Schema defines `skip_integrity_check: bool` 
- ✅ Frontend UI renders checkbox and tries to send message
- ❌ Bridge message variant NOT DEFINED
- ❌ Backend handler NOT IMPLEMENTED

**Impact**: Toggle works in UI state but changes are NOT persisted to backend or disk

**Resolution Needed**: 
1. Add variant to `MessageToBackend` enum in [crates/bridge/src/message.rs](crates/bridge/src/message.rs)
2. Add handler in [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs)

---

