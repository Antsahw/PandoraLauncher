# PandoraLauncher - Complete Codebase Analysis

## 1. PROJECT STRUCTURE OVERVIEW

### Workspace Organization
PandoraLauncher is a **multi-crate Rust workspace** using a modular architecture:

```
crates/
├── pandora_launcher/     # Main binary entry point
├── backend/              # Core launcher logic & Java process execution
├── frontend/             # UI layer (GPUI-based)
├── bridge/               # Messages & types shared between frontend/backend
├── schema/               # Data structures & serialization (JSON/TOML)
├── auth/                 # Minecraft authentication (Microsoft OAuth2)
├── reqwest_client/       # HTTP client wrapper
├── nbt/                  # NBT format parsing (Minecraft binary format)
└── ftree/                # File tree utilities
```

**Workspace Configuration**
- Edition: 2024
- Default member: `crates/pandora_launcher`
- Multi-threaded build with LTO enabled for releases

---

## 2. JAVA INVOCATION PATTERNS

### Primary Java Execution (Instances)
**File**: [crates/backend/src/launch.rs](crates/backend/src/launch.rs#L168)

```rust
let (java_path, assets_index_name, library_paths, log_configuration) = tokio::select! {
    // Loads Mojang's Java runtime or uses custom JVM
};

// Builds classpath from libraries, then invokes Java with:
let child = Command::new(java_path)
    .arg("-Xmx{max}M")
    .arg("-Xms{min}M")
    .args(jvm_flags)
    .arg("-cp")
    .arg(classpath)
    .env("LD_LIBRARY_PATH", natives_dir)  // Native libraries
    .current_dir(&dot_minecraft_path)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?
```

### Server Execution (Minecraft Servers)
**File**: [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs#L2323)

Three major server types handled:

#### 1. **Forge Servers**
```rust
let mut cmd = std::process::Command::new("java");
// Uses Forge-specific argument files if available:
// - user_jvm_args.txt (user settings)
// - libraries/net/minecraftforge/forge/{version}-{build}/unix_args.txt
cmd.arg("@user_jvm_args.txt")
   .arg(format!("@libraries/.../unix_args.txt"))
   .arg("nogui")
```

#### 2. **NeoForge Servers** 
```rust
// NeoForge installer creates run.sh script with proper JVM arguments
std::fs::set_permissions(&run_script, Permissions::from_mode(0o755));  // Unix
let cmd = std::process::Command::new("bash");
cmd.arg(&run_script);
```

#### 3. **Standard Servers** (Paper, Purpur, Fabric, Vanilla)
```rust
cmd.arg("-Xmx1024M")
   .arg("-Xms512M")
   .arg("-jar")
   .arg(&jar_path)
   .arg("nogui")
```

### Java Binary Resolution
**File**: [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs#L2186)

Mojang's Java runtime is downloaded and extracted. Platform-specific paths:
- **Windows**: `bin/javaw.exe`, `MinecraftJava.exe`
- **Linux**: `bin/java`
- **macOS**: `jre.bundle/Contents/Home/bin/java`

Java version selection logic:
```rust
pub fn get_recommended_java_version(minecraft_version: &str) -> u32 {
    // 1.0-1.11:   Java 8 (minimum)
    // 1.12-1.16:  Java 8
    // 1.17-1.20:  Java 16/17
    // 1.21+:      Java 21
}
```

Process Tracking:
```rust
self.server_processes.write().insert(name, child);  // HashMap<String, std::process::Child>
self.server_game_output_ids.write().insert(name, output_id);  // Captures stdout/stderr
```

---

## 3. CONFIGURATION SYSTEM

### Configuration File Structure

#### Instance Configuration
**Location**: `{launcher_dir}/instances/{name}/.minecraft/`
**File**: `../info_v1.json` (parent directory)

```rust
#[derive(Serialize, Deserialize)]
pub struct InstanceConfiguration {
    pub minecraft_version: Ustr,
    pub loader: Loader,  // Vanilla | Fabric | Forge | NeoForge
    pub preferred_loader_version: Option<Ustr>,
    pub preferred_account: Option<Uuid>,
    
    // Runtime Configuration
    pub memory: Option<InstanceMemoryConfiguration>,
    pub wrapper_command: Option<InstanceWrapperCommandConfiguration>,
    pub jvm_flags: Option<InstanceJvmFlagsConfiguration>,
    pub jvm_binary: Option<InstanceJvmBinaryConfiguration>,
    pub linux_wrapper: Option<InstanceLinuxWrapperConfiguration>,
    pub system_libraries: Option<InstanceSystemLibrariesConfiguration>,
    pub instance_fallback_icon: Option<Ustr>,
    pub disable_file_syncing: bool,
}
```

#### Memory Configuration
```rust
pub struct InstanceMemoryConfiguration {
    pub enabled: bool,
    pub min: u32,  // MiB (default: 512)
    pub max: u32,  // MiB (default: 4096)
}
```

#### JVM Binary Configuration
```rust
pub struct InstanceJvmBinaryConfiguration {
    pub enabled: bool,
    pub path: Option<Arc<Path>>,  // Custom Java executable path
    pub forced_java_version: Option<u32>,  // Override default (8, 11, 17, 21)
}
```

#### JVM Flags Configuration
```rust
pub struct InstanceJvmFlagsConfiguration {
    pub enabled: bool,
    pub flags: Arc<str>,  // Custom JVM arguments
}
```

#### Linux Wrapper Configuration
```rust
pub struct InstanceLinuxWrapperConfiguration {
    pub use_mangohud: bool,
    pub use_gamemode: bool,
    pub use_discrete_gpu: bool,
    pub disable_gl_threaded_optimizations: bool,
}
```

#### Backend Configuration
**Location**: `{launcher_dir}/backend_config.json`

```rust
pub struct BackendConfig {
    pub sync_targets: SyncTargets,
    pub dont_open_game_output_when_launching: bool,
    pub proxy: ProxyConfig,
    pub java_runtimes: JavaRuntimesConfig,
}

pub struct JavaRuntimesConfig {
    pub default_java_version: u32,  // Global default (e.g., 8, 11, 17, 21)
}

pub struct ProxyConfig {
    pub enabled: bool,
    pub protocol: ProxyProtocol,  // Http | Https | Socks5
    pub host: String,
    pub port: u16,
    pub auth_enabled: bool,
    pub username: String,
}
```

### Configuration Persistence
**File**: [crates/backend/src/persistent.rs](crates/backend/src/persistent.rs)

Uses the `Persistent<T>` generic wrapper:

```rust
pub struct Persistent<T: Serialize + Deserialize> {
    path: Arc<Path>,
    dirty: bool,
    data: T,
}

impl<T> Persistent<T> {
    pub fn load(path: Arc<Path>) -> Self {
        let data = read_json(&path).unwrap_or_default();
        Self { path, dirty: false, data }
    }
    
    pub fn modify(&mut self, func: impl FnOnce(&mut T)) {
        if self.dirty { self.load_from_disk(); }
        func(&mut self.data);
        
        // Atomic write with temp file
        if let Ok(bytes) = serde_json::to_vec(&self.data) {
            write_safe(&self.path, &bytes);
        }
    }
}
```

**Safe Writing Pattern** ([crates/backend/src/lib.rs](crates/backend/src/lib.rs#L68)):
```rust
pub fn write_safe(path: &Path, content: &[u8]) -> std::io::Result<()> {
    // 1. Write to temp file with random suffix
    let mut temp = path.with_added_suffix(format!("{}.new", random_u32()));
    
    // 2. Flush & sync all
    temp_file.write_all(content)?;
    temp_file.sync_all()?;
    
    // 3. Atomic rename
    std::fs::rename(&temp, path)?
}
```

### Configuration Locations

| Component | Location | Format |
|-----------|----------|--------|
| Instance Config | `{instance}/../info_v1.json` | JSON |
| Backend Config | `{launcher_dir}/backend_config.json` | JSON |
| Accounts | `{launcher_dir}/accounts.json` | JSON |
| Interface Config | `{launcher_dir}/interface_config.json` | JSON |
| Metadata Cache | `{launcher_dir}/.metadata/` | JSON |

### Server Configuration Files
**Location**: `{launcher_dir}/instances/{name}/.minecraft/`

| File | Purpose |
|------|---------|
| `server.properties` | Standard Minecraft server config |
| `user_jvm_args.txt` | Forge user JVM arguments |
| `run.sh` | NeoForge startup script |
| `logs/` | Server output logs |
| `libraries/` | Forge/mod libraries |

---

## 4. ENVIRONMENT VARIABLES & SYSTEM PATHS

### Set During Launch
**File**: [crates/backend/src/launch.rs](crates/backend/src/launch.rs#L206)

```rust
// Critical environment variables set:
env("LD_LIBRARY_PATH", natives_dir)     // Native JNI libraries extracted from JARs
env("JAVA_LIBRARY_PATH", natives_dir)   // Alternative path for natives

// Game-specific:
env("APPDATA", {game_dir})  // Windows Minecraft config

// User variables passed through:
env("CLASSPATH", classpath)  // All JARs
env("JAVA_HOME", java_binary_dir)  // Optional

// Minecraft-specific variables substituted in launch args:
env_vars.insert("${auth_player_name}", username)
env_vars.insert("${version_name}", version)
env_vars.insert("${game_directory}", dot_minecraft_path)
env_vars.insert("${assets_root}", assets_dir)
env_vars.insert("${assets_index_name}", assets_index_version)
env_vars.insert("${auth_uuid}", uuid)
env_vars.insert("${auth_access_token}", token)
env_vars.insert("${clientid}", session_id)
env_vars.insert("${auth_xuid}", xuid)
env_vars.insert("${user_properties}", json_encoded_properties)
```

### Directory Structure

```
{launcher_dir}/
├── instances/
│   └── {instance_name}/
│       ├── info_v1.json              (InstanceConfiguration)
│       ├── icon.png
│       └── .minecraft/
│           ├── saves/                (Worlds)
│           ├── servers.dat           (Server list - NBT format)
│           ├── resourcepacks/
│           ├── mods/
│           ├── logs/
│           ├── options.txt
│           └── (Minecraft data files)
├── libraries/                        (Maven libraries)
├── assets/
│   ├── indexes/
│   ├── objects/                      (Asset files)
│   └── virtual/
├── versions/
├── runtime/                          (Java runtimes)
├── .metadata/                        (Downloaded metadata caches)
├── accounts.json
├── backend_config.json
├── interface_config.json
└── temp/
```

### System Library Paths (Linux Specific)
**File**: [crates/schema/src/instance.rs](crates/schema/src/instance.rs#L119)

```rust
pub struct InstanceSystemLibrariesConfiguration {
    pub lwjgl_library_path: LwjglLibraryPath,
    // Auto (system libraries)
    // AutoPreferred(custom_path)
    // Explicit(specified_path)
}
```

---

## 5. KEY MODULES INVOLVED IN RUNTIME MANAGEMENT

### Core Backend State
**File**: [crates/backend/src/backend.rs](crates/backend/src/backend.rs#L210)

```rust
pub struct BackendState {
    pub launcher: Launcher,                 // Instance/game launch logic
    pub instance_state: Arc<RwLock<BackendStateInstances>>,
    pub server_processes: Arc<RwLock<HashMap<String, std::process::Child>>>,
    pub server_game_output_ids: Arc<RwLock<HashMap<String, GameOutputId>>>,
    pub mod_metadata_manager: Arc<ModMetadataManager>,
    pub account_info: Arc<RwLock<AccountInfo>>,
    pub config: Arc<RwLock<Persistent<BackendConfig>>>,
    pub meta: Arc<MetadataManager>,
    pub directories: Arc<LauncherDirectories>,
}
```

### Instance Management
**File**: [crates/backend/src/instance.rs](crates/backend/src/instance.rs#L32)

```rust
pub struct Instance {
    pub id: InstanceID,
    pub root_path: Arc<Path>,
    pub dot_minecraft_path: Arc<Path>,
    pub configuration: Persistent<InstanceConfiguration>,
    pub processes: Vec<std::process::Child>,
    pub game_output_id: Option<GameOutputId>,
    
    // Content tracking
    pub mod_state: ContentFolderState,
    pub resource_pack_state: ContentFolderState,
    pub shader_pack_state: ContentFolderState,
}

impl Instance {
    pub fn load_from_folder(path: impl AsRef<Path>) -> Result<Self, InstanceLoadError> {
        // Loads info_v1.json, discovers .minecraft, sets up watchers
    }
}
```

### Launch System
**File**: [crates/backend/src/launch.rs](crates/backend/src/launch.rs#L33)

```rust
pub struct Launcher {
    meta: Arc<MetadataManager>,
    directories: Arc<LauncherDirectories>,
    launch_wrapper: Arc<Path>,
    sender: FrontendHandle,
    config: Arc<RwLock<Persistent<BackendConfig>>>,
}

impl Launcher {
    pub async fn launch(
        &self,
        dot_minecraft_path: Arc<Path>,
        instance_info: InstanceConfiguration,
        quick_play: Option<QuickPlayLaunch>,
        login_info: MinecraftLoginInfo,
        add_mods: Vec<PathBuf>,
    ) -> Result<Child, LaunchError> {
        // 1. Load Java runtime (Mojang or custom)
        // 2. Resolve library paths
        // 3. Extract natives
        // 4. Download assets
        // 5. Construct JVM command
        // 6. Spawn process
    }
}
```

### Game Output Capture
**File**: [crates/backend/src/log_reader.rs](crates/backend/src/log_reader.rs)

```rust
pub fn start_game_output(
    stdout: std::process::ChildStdout,
    stderr: Option<std::process::ChildStderr>,
    send: FrontendHandle,
    game_name: String,
) -> GameOutputId {
    // Spawns background task reading process stdout/stderr
    // Sends OutputLine messages to frontend UI in real-time
}
```

### Server Management
**File**: [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs)

Handles start_server action:
1. Downloads server JAR if needed
2. Accepts memory/JVM settings from `ServerSettingsSubpage`
3. Executes Java command with appropriate flags
4. Stores in `server_processes` HashMap
5. Starts `GameOutput` for console viewing

### Loader Support
**File**: [crates/schema/src/loader.rs](crates/schema/src/loader.rs)

```rust
#[derive(Serialize, Deserialize, Hash)]
pub enum Loader {
    Vanilla,
    Fabric,
    Forge,
    NeoForge,
    Unknown,
}
```

Each loader has unique installation/configuration paths handled in:
- [crates/backend/src/launch.rs](crates/backend/src/launch.rs) - Library resolution
- [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs) - Server execution

---

## 6. FRONTEND PAGES FOR CONFIGURATION

### Instance Settings UI
**File**: [crates/frontend/src/pages/instance/settings_subpage.rs](crates/frontend/src/pages/instance/settings_subpage.rs)

- Memory settings slider (min/max)
- JVM flags text input
- Custom Java binary path picker
- Wrapper command configuration
- Linux-specific options (MangoHUD, GameMode, etc.)

### Server Settings UI
**File**: [crates/frontend/src/pages/instance/server_settings_subpage.rs](crates/frontend/src/pages/instance/server_settings_subpage.rs)

- Server memory configuration
- Properties file editor
- Log viewer

### Server Page
**File**: [crates/frontend/src/pages/instance/server_page.rs](crates/frontend/src/pages/instance/server_page.rs)

- Server start/stop controls
- Real-time log output
- Server properties editor
- EULA acceptance

---

## 7. DATA FLOW: LOADING & PERSISTING SETTINGS

```
Frontend UI Input
    ↓
ModalAction Message (e.g., UpdateInstanceConfig)
    ↓
BackendHandler receives message
    ↓
Instance.configuration.modify(|config| {
    config.memory.max = new_value;
})
    ↓
Persistent<T>::modify() {
    → write_safe() to {instance}/info_v1.json
    → Atomic temp file + rename
}
    ↓
WatchTarget::InstanceDir detects change
    ↓
Frontend reloads from Instance::configuration
```

---

## 8. KEY STRUCTS FOR RUNTIME

| Struct | Purpose | Location |
|--------|---------|----------|
| `InstanceConfiguration` | Instance settings persistence | schema/src/instance.rs |
| `BackendConfig` | Global launcher settings | schema/src/backend_config.rs |
| `Launcher` | Instance/game launch orchestrator | backend/src/launch.rs |
| `Instance` | In-memory instance representation | backend/src/instance.rs |
| `BackendState` | Main backend state container | backend/src/backend.rs |
| `Persistent<T>` | Configuration file wrapper | backend/src/persistent.rs |
| `GameOutput` | Real-time game console | frontend/src/game_output/ |
| `InstanceMemoryConfiguration` | JVM heap settings | schema/src/instance.rs |
| `InstanceJvmBinaryConfiguration` | Java runtime selection | schema/src/instance.rs |
| `InstanceJvmFlagsConfiguration` | Custom JVM arguments | schema/src/instance.rs |

---

## 9. SUMMARY: JAVA INVOCATION WORKFLOW

```
┌─────────────────────────────────────────────────────────────┐
│ User clicks "Launch" in Instance                             │
└────────────────────┬────────────────────────────────────────┘
                     ↓
          ┌──────────────────────┐
          │ Load InstanceConfig  │
          │ (from info_v1.json)  │
          └───────────┬──────────┘
                      ↓
         ┌────────────────────────────┐
         │ Resolve Java Runtime       │
         │ - Check forced_java_version│
         │ - Use Mojang's runtime     │
         │ - Or system Java           │
         └────────────┬───────────────┘
                      ↓
         ┌────────────────────────────┐
         │ Load Libraries & Assets    │
         │ - Parse version manifest   │
         │ - Download missing libs    │
         │ - Extract natives          │
         └────────────┬───────────────┘
                      ↓
         ┌────────────────────────────┐
         │ Build JVM Command          │
         │ java -Xmx{max} -Xms{min}  │
         │   {jvm_flags}              │
         │   -cp {classpath}          │
         │   net.minecraft.client...  │
         │   {auth args}              │
         └────────────┬───────────────┘
                      ↓
         ┌────────────────────────────┐
         │ Set Environment Variables  │
         │ - LD_LIBRARY_PATH (natives)│
         │ - Auth tokens              │
         │ - Game paths               │
         └────────────┬───────────────┘
                      ↓
         ┌────────────────────────────┐
         │ Spawn Process              │
         │ std::process::Command      │
         └────────────┬───────────────┘
                      ↓
         ┌────────────────────────────┐
         │ Capture stdout/stderr      │
         │ Send to UI (GameOutput)    │
         └────────────────────────────┘
```

**For Servers**: Direct JAR execution with `-Xmx`/`-Xms` or NeoForge runtime script.

