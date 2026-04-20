# Codebase Search Results: Game Instance Launch & Configuration

## 1. GAME INSTANCE LAUNCH CODE

### Primary Launch Location
- **File:** [crates/backend/src/launch.rs](crates/backend/src/launch.rs)
- **Main Launch Function:** `LaunchContext::launch()` 
- **Line Numbers:** [2070-2170](crates/backend/src/launch.rs#L2070-L2170)

### Launch Process Flow

#### Step 1: Java Command Setup (Line 2109)
```rust
std::process::Command::new(&*self.java_path)
```
- Creates the Java process command at [line 2109](crates/backend/src/launch.rs#L2109)
- `self.java_path` is resolved from instance configuration and Java runtime settings

#### Step 2: Wrapper Command Injection (Lines 2099-2108)
- [Lines 2099-2108](crates/backend/src/launch.rs#L2099-L2108): Wrapper command configuration (mangohud, gamemode, custom wrappers)
- If wrapper command enabled, wraps the Java binary:
  ```rust
  cmd.arg(&*self.java_path);  // Add Java path as argument to wrapper
  ```

#### Step 3: JVM Arguments Assembly (Lines 2130-2165)
- **Memory flags:** [Lines 2143-2145](crates/backend/src/launch.rs#L2143-L2145)
  - Reads from `self.configuration.memory`
  - Adds `-Xms{min}m -Xmx{max}m` arguments

- **JVM Flags:** [Lines 2146-2151](crates/backend/src/launch.rs#L2146-L2151)
  - Reads from `self.configuration.jvm_flags`
  - Parses shell_words or splits whitespace
  - Adds custom flags to command

- **Launch Wrapper:** [Line 2153](crates/backend/src/launch.rs#L2153)
  - Main class: `com.moulberry.pandora.LaunchWrapper`

#### Step 4: Process Spawn (Line 2157)
```rust
let mut child = command.spawn()?;
```
- Actually executes the Java process at [line 2157](crates/backend/src/launch.rs#L2157)

### Linux-Specific Wrappers (Lines 2119-2127)
- `DRI_PRIME=1` for discrete GPU usage at [line 2121](crates/backend/src/launch.rs#L2121)
- `__GL_THREADED_OPTIMIZATIONS=0` flag control at [line 2123](crates/backend/src/launch.rs#L2123)

---

## 2. INSTANCE CONFIGURATION DATA STRUCTURE

### InstanceConfiguration Definition
- **File:** [crates/schema/src/instance.rs](crates/schema/src/instance.rs)
- **Lines:** [11-41](crates/schema/src/instance.rs#L11-L41)

### Key Configuration Fields

| Field | Type | Location | Purpose |
|-------|------|----------|---------|
| `minecraft_version` | `Ustr` | Line 12 | Game version (e.g., "1.20.1") |
| `loader` | `Loader` | Line 13 | Vanilla/Fabric/Forge/etc |
| `memory` | `InstanceMemoryConfiguration` | Lines 17-18 | Min/max heap size |
| `wrapper_command` | `InstanceWrapperCommandConfiguration` | Lines 19-20 | Pre-java wrapper (mangohud, etc) |
| `jvm_flags` | `InstanceJvmFlagsConfiguration` | Lines 21-22 | Custom JVM flags (-Xmx, etc) |
| `jvm_binary` | `InstanceJvmBinaryConfiguration` | Lines 23-24 | Custom Java binary path |
| `java_runtime` | `InstanceJavaRuntimeConfiguration` | Lines 25-26 | Per-instance Java runtime selection |
| `linux_wrapper` | `InstanceLinuxWrapperConfiguration` | Lines 27-28 | Linux-specific: mangohud, gamemode |
| `system_libraries` | `InstanceSystemLibrariesConfiguration` | Lines 29-30 | GLFW/OpenAL library paths |

### Sub-Configurations

#### InstanceMemoryConfiguration
- **Lines:** [74-90](crates/schema/src/instance.rs#L74-L90)
- Fields: `enabled`, `min` (default 512MB), `max` (default 4096MB)

#### InstanceJvmFlagsConfiguration
- **Lines:** [104-109](crates/schema/src/instance.rs#L104-L109)
- Fields: `enabled`, `flags` (Arc<str> for custom flags)

#### InstanceJavaRuntimeConfiguration
- **Lines:** [135-149](crates/schema/src/instance.rs#L135-L149)
- Fields: `enabled`, `runtime_name` (e.g., "java21", "system")

#### InstanceLinuxWrapperConfiguration
- **Lines:** [150-170](crates/schema/src/instance.rs#L150-L170)
- Fields: `use_mangohud`, `use_gamemode`, `use_discrete_gpu`, `disable_gl_threaded_optimizations`

---

## 3. INSTANCE CONFIGURATION LOADING

### Load From Disk
- **File:** [crates/backend/src/backend.rs](crates/backend/src/backend.rs)
- **Location:** [Lines 62-76](crates/backend/src/backend.rs#L62-L76)
- **Config File:** `{launcher_root}/config.json` for backend config
- **Instance Files:** `{launcher_root}/instances/{instance_id}/info_v1.json`

### Configuration Persistence
- **File:** [crates/schema/src/persistence.rs](crates/schema/src)
- Uses `Persistent<T>` wrapper for auto-save on modification
- `.modify()` callback modifies and immediately saves config

### Configuration Update Flow (Backend Handler)
- **File:** [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs)
- **Lines:** [280-310](crates/backend/src/backend_handler.rs#L280-L310)
- Receives messages and updates instance configuration:
  ```rust
  MessageToBackend::SetInstanceJvmFlags { id, jvm_flags } => {
      if let Some(instance) = self.instance_state.write().instances.get_mut(id) {
          instance.configuration.modify(|configuration| {
              configuration.jvm_flags = Some(jvm_flags);
          });
      }
  }
  ```

---

## 4. FRONTEND DIRECTORY STRUCTURE

### Overview
```
crates/frontend/src/
├── pages/
│   ├── instance/
│   │   ├── instance_page.rs           (Main instance display)
│   │   ├── settings_subpage.rs        (Instance settings - MAIN CONFIG UI)
│   │   ├── mods_subpage.rs
│   │   ├── resource_packs_subpage.rs
│   │   ├── server_page.rs
│   │   ├── quickplay_subpage.rs
│   │   ├── logs_subpage.rs
│   │   └── mod.rs
│   ├── instances_page.rs              (All instances list)
│   ├── servers_page.rs
│   └── ...
├── components/
│   ├── java_runtime_config.rs         (Java runtime UI components)
│   └── ...
└── ...
```

### Settings UI Component
- **File:** [crates/frontend/src/pages/instance/settings_subpage.rs](crates/frontend/src/pages/instance/settings_subpage.rs)
- This is the PRIMARY location where instance settings are displayed and modified
- Contains UI for: memory, wrapper command, JVM flags, Java binary, Linux wrappers, system libraries

#### Settings Subpage State (Lines 40-70)
```rust
pub struct SettingsSubpage {
    memory_override_enabled: bool,
    memory_min_input_state: Entity<InputState>,
    memory_max_input_state: Entity<InputState>,
    wrapper_command_enabled: bool,
    wrapper_command_input_state: Entity<InputState>,
    // ... more fields for other settings
}
```

#### UI for Different Configuration Types

| Setting | Line Range | Field Name |
|---------|-----------|-----------|
| Memory Min/Max | ~170-200 | `memory_{min\|max}_input_state` |
| Wrapper Command | ~205-230 | `wrapper_command_input_state` |
| JVM Flags | ~240-280 | `jvm_flags_input_state` |
| Java Binary Path | ~290-330 | Custom path selection |
| Linux Wrappers | ~350-400 | Checkbox toggles for mangohud/gamemode |

### Java Runtime Configuration Component
- **File:** [crates/frontend/src/components/java_runtime_config.rs](crates/frontend/src/components/java_runtime_config.rs)
- UI panels for selecting Java runtime
- Lists available Java versions
- Shows default runtime selection

---

## 5. FRONTEND-BACKEND COMMUNICATION

### Message Architecture
- **Bridge File:** [crates/bridge/src/message.rs](crates/bridge/src/message.rs)
- Defines `MessageToBackend` enum with all configuration update messages

### Configuration Update Messages

#### Memory Configuration
- **Message:** `SetInstanceMemory`
- **Struct:** `InstanceMemoryConfiguration { enabled: bool, min: u32, max: u32 }`
- **Sent From:** [settings_subpage.rs ~Line 560](crates/frontend/src/pages/instance/settings_subpage.rs#L560)
- **Received At:** [backend_handler.rs ~Line 285](crates/backend/src/backend_handler.rs#L285)

#### JVM Flags
- **Message:** `SetInstanceJvmFlags`
- **Line:** [bridge/message.rs ~Line 108](crates/bridge/src/message.rs#L108)
- **Sent From:** [settings_subpage.rs ~Line 567](crates/frontend/src/pages/instance/settings_subpage.rs#L567)
- **Received At:** [backend_handler.rs ~Line 291](crates/backend/src/backend_handler.rs#L291)

#### JVM Binary (Custom Java path)
- **Message:** `SetInstanceJvmBinary`
- **Line:** [bridge/message.rs ~Line 112](crates/bridge/src/message.rs#L112)
- **Struct:** `InstanceJvmBinaryConfiguration { enabled: bool, path: Option<Arc<Path>>, forced_java_version: Option<u32> }`
- **Received At:** [backend_handler.rs ~Line 297](crates/backend/src/backend_handler.rs#L297)

#### Linux Wrappers
- **Message:** `SetInstanceLinuxWrapper`
- **Struct:** `InstanceLinuxWrapperConfiguration { use_mangohud, use_gamemode, use_discrete_gpu, disable_gl_threaded_optimizations }`
- **Received At:** [backend_handler.rs ~Line 303](crates/backend/src/backend_handler.rs#L303)

#### System Libraries
- **Message:** `SetInstanceSystemLibraries`
- **Struct:** `InstanceSystemLibrariesConfiguration { override_glfw, glfw, override_openal, openal }`
- **Received At:** [backend_handler.rs ~Line 309](crates/backend/src/backend_handler.rs#L309)

#### Wrapper Command (Pre-Java wrapper)
- **Message:** `SetInstanceWrapperCommand`
- **Received At:** [backend_handler.rs ~Line 285](crates/backend/src/backend_handler.rs#L285)

### Backend Processing
- **File:** [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs)
- **Lines:** [280-315](crates/backend/src/backend_handler.rs#L280-L315)
- All messages follow pattern:
  1. Extract instance from state
  2. Call `.modify()` on `Persistent<InstanceConfiguration>`
  3. Update affects running instances via launch context

### Frontend Event Handler Example
From [settings_subpage.rs](crates/frontend/src/pages/instance/settings_subpage.rs):

```rust
// Send JVM flags to backend
self.backend_handle.send(MessageToBackend::SetInstanceJvmFlags {
    id: self.instance_id,
    jvm_flags: InstanceJvmFlagsConfiguration {
        enabled: self.jvm_flags_enabled,
        flags: Arc::from(self.jvm_flags_input.to_string()),
    }
});
```

### Communication Flow Diagram
```
Frontend (settings_subpage.rs)
    ↓
emit MessageToBackend (via backend_handle)
    ↓
Backend Handler (backend_handler.rs)
    ↓
Update InstanceConfiguration (Persistent<T>)
    ↓
Auto-save to {launcher_root}/instances/{id}/info_v1.json
    ↓
On launch: LaunchContext uses updated configuration
    ↓
Java process spawned with updated flags/memory
```

---

## 6. QUICK REFERENCE: WHERE TO MODIFY

### To add a new instance setting:
1. Add field to `InstanceConfiguration` in [crates/schema/src/instance.rs](crates/schema/src/instance.rs)
2. Add `MessageToBackend::SetInstance*` variant in [crates/bridge/src/message.rs](crates/bridge/src/message.rs)
3. Add handler in [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs) ~Line 280-310
4. Add UI in [crates/frontend/src/pages/instance/settings_subpage.rs](crates/frontend/src/pages/instance/settings_subpage.rs)
5. Use in `LaunchContext::launch()` at [crates/backend/src/launch.rs ~Line 2070+](crates/backend/src/launch.rs#L2070)

### To modify Java launch arguments:
- Edit `LaunchContext::launch()` at [crates/backend/src/launch.rs](crates/backend/src/launch.rs#L2070-L2170)
- Arguments processing at [lines 2130-2160](crates/backend/src/launch.rs#L2130-L2160)

### To add Java runtime detection/selection:
- Edit [crates/backend/src/java_runtime.rs](crates/backend/src/java_runtime.rs) (if exists)
- Update UI in [crates/frontend/src/components/java_runtime_config.rs](crates/frontend/src/components/java_runtime_config.rs)
- Ensure `InstanceJavaRuntimeConfiguration` is used at launch time
