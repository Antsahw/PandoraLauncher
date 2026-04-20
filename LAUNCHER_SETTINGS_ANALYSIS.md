# Launcher-Level Settings/Configuration - Complete Analysis

## Overview
The PandoraLauncher has two main categories of launcher-level (application-wide) settings:
1. **Backend Configuration** - Server/network settings
2. **Frontend Configuration** - UI/Interface settings

---

## 1. BACKEND CONFIGURATION

### Location & File Structure

**File Path:** [crates/schema/src/backend_config.rs](crates/schema/src/backend_config.rs)

**Persisted File:**
- Location: `{launcher_root}/config.json`
- Format: JSON
- Loader: [crates/backend/src/backend.rs#L76](crates/backend/src/backend.rs#L76)
  ```rust
  let mut config: Persistent<BackendConfig> = Persistent::load(directories.config_json.clone());
  ```

### Data Structure Definition

```rust
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct BackendConfig {
    #[serde(default, skip_serializing_if = "is_default_sync_targets", deserialize_with = "try_deserialize_sync_targets")]
    pub sync_targets: SyncTargets,
    
    #[serde(default, skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub dont_open_game_output_when_launching: bool,
    
    #[serde(default, skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub proxy: ProxyConfig,
}
```

### Sub-structures

#### SyncTargets
```rust
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct SyncTargets {
    pub files: BTreeSet<Arc<str>>,      // Files to sync (e.g., "options.txt", "servers.dat")
    pub folders: BTreeSet<Arc<str>>,    // Folders to sync (e.g., "saves", "config", "screenshots")
}
```

**Legacy Migration:** Supports migration from previous bitset format through `try_deserialize_sync_targets`

**Sync Target Examples:**
- Files: `options.txt`, `servers.dat`, `command_history.txt`, `hotbar.nbt`
- Folders: `saves`, `config`, `screenshots`, `resourcepacks`, `shaderpacks`

#### ProxyConfig
```rust
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ProxyConfig {
    pub enabled: bool,                  // Whether proxy is enabled
    pub protocol: ProxyProtocol,        // HTTP, HTTPS, or SOCKS5
    pub host: String,                   // Proxy hostname/IP
    pub port: u16,                      // Proxy port
    pub auth_enabled: bool,             // Whether authentication is required
    pub username: String,               // Proxy username
    // Password is stored separately in system keyring (see PlatformSecretStorage)
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ProxyProtocol {
    #[default]
    Http,       // HTTP proxy
    Https,      // HTTPS proxy
    Socks5,     // SOCKS5 proxy
}
```

### Configuration Loading

**Initialization:** [crates/backend/src/backend.rs#L62-L76](crates/backend/src/backend.rs#L62-L76)

```rust
let directories = Arc::new(LauncherDirectories::new(launcher_dir));

let mut config: Persistent<BackendConfig> = Persistent::load(directories.config_json.clone());
let proxy_config = config.get().proxy.clone();

// Proxy password is loaded separately from system keyring/secret storage
let proxy_password: Option<String> = if proxy_config.enabled && proxy_config.auth_enabled {
    runtime.block_on(async {
        match PlatformSecretStorage::new().await {
            Ok(storage) => match storage.read_proxy_password().await {
                Ok(password) => password,
                Err(e) => {
                    log::warn!("Failed to read proxy password from keyring: {:?}", e);
                    None
                }
            },
            // ...
        }
    })
} else {
    None
};
```

### Configuration Updates

**Message Handler:** [crates/backend/src/backend_handler.rs#L1519-L1549](crates/backend/src/backend_handler.rs#L1519-L1549)

#### Update Game Output Flag
```rust
MessageToBackend::SetOpenGameOutputAfterLaunching { value } => {
    let mut config = self.config.write();
    config.modify(|config| {
        config.dont_open_game_output_when_launching = !value;
    });
}
```

#### Update Proxy Configuration
```rust
MessageToBackend::SetProxyConfiguration { config, password } => {
    let mut backend_config = self.config.write();
    backend_config.modify(|backend_config| {
        backend_config.proxy = config;
    });
    
    // Handle password storage
    if !password.is_empty() {
        // Store in system keyring
        runtime.block_on(async {
            match PlatformSecretStorage::new().await {
                Ok(storage) => {
                    if password.is_empty() {
                        if let Err(e) = storage.delete_proxy_password().await {
                            log::warn!("Failed to delete proxy password from keyring: {:?}", e);
                        }
                    } else if let Err(e) = storage.write_proxy_password(&password).await {
                        log::warn!("Failed to write proxy password to keyring: {:?}", e);
                        self.send.send_error("Failed to save proxy password to system keyring");
                    }
                }
                // ...
            }
        });
    }
}
```

#### Retrieve Configuration
```rust
MessageToBackend::GetBackendConfiguration { channel } => {
    let configuration = self.config.write().get().clone();
    
    let proxy_password = if configuration.proxy.enabled && configuration.proxy.auth_enabled {
        // Fetch from system keyring
        // ...
    } else {
        None
    };
    
    let _ = channel.send(BackendConfigWithPassword {
        config: configuration,
        proxy_password,
    });
}
```

---

## 2. FRONTEND CONFIGURATION

### Location & File Structure

**File Path:** [crates/frontend/src/interface_config.rs](crates/frontend/src/interface_config.rs)

**Persisted File:**
- Location: `{launcher_root}/interface.json`
- Format: JSON
- Loader: [crates/frontend/src/lib.rs#L114](crates/frontend/src/lib.rs#L114)
  ```rust
  InterfaceConfig::init(cx, launcher_dir.join("interface.json").into());
  ```

### Data Structure Definition

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct InterfaceConfig {
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub active_theme: SharedString,                     // Active UI theme
    
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub main_window_bounds: WindowBounds,               // Main window position/size
    
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub game_output_bounds: WindowBounds,               // Game output window position/size
    
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub sidebar_width: f32,                             // Sidebar width preference
    
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub main_page: PageType,                            // Currently active main page
    
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub page_path: Arc<[PageType]>,                     // Page navigation path
    
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub quick_delete_mods: bool,                        // Skip confirmation when deleting mods
    
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub quick_delete_instance: bool,                    // Skip confirmation when deleting instances
    
    #[serde(default = "schema::default_true", deserialize_with = "schema::try_deserialize")]
    pub content_install_latest: bool,                   // Default: true - Install latest version
    
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub content_filter_version: bool,                   // Filter content by version
    
    #[serde(default = "default_modrinth_project_type", deserialize_with = "schema::try_deserialize")]
    pub modrinth_page_project_type: ModrinthProjectType, // Default: Mod - Modrinth page filter
    
    #[serde(default = "default_curseforge_class_id", deserialize_with = "schema::try_deserialize")]
    pub curseforge_page_class_id: CurseforgeClassId,   // Default: Mod - CurseForge page filter
    
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub hide_main_window_on_launch: bool,               // Hide launcher when starting game
    
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub quit_on_main_closed: bool,                      // Close all windows when main closes
    
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub hide_server_addresses: bool,                    // Privacy setting
    
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub show_snapshots_in_create_instance: bool,        // Include snapshots in version list
    
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub instances_view_mode: InstancesViewMode,         // Cards or List view
    
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub servers_view_mode: ServersViewMode,             // Cards or List view
    
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub instance_subpage: InstanceSubpageType,          // Active instance settings subpage
    
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub server_subpage: ServerSubpageType,              // Active server settings subpage
}
```

### Sub-structures

#### WindowBounds
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WindowBounds {
    #[default]
    Inherit,                    // Use platform default window behavior
    Windowed { x: f32, y: f32, w: f32, h: f32 },  // Floating window
    Maximized { x: f32, y: f32, w: f32, h: f32 }, // Maximized state
    Fullscreen { x: f32, y: f32, w: f32, h: f32 }, // Fullscreen state
}
```

#### View Modes
```rust
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, strum::EnumIter)]
pub enum InstancesViewMode {
    #[default]
    Cards,  // Tile/card view
    List,   // List view
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, strum::EnumIter)]
pub enum ServersViewMode {
    #[default]
    Cards,  // Tile/card view
    List,   // List view
}
```

### Configuration Initialization & Access

**Initialization:** [crates/frontend/src/interface_config.rs#L198-L207](crates/frontend/src/interface_config.rs#L198-L207)

```rust
impl InterfaceConfig {
    pub fn init(cx: &mut App, path: Arc<Path>) {
        cx.set_global(InterfaceConfigHolder {
            config: try_read_json(&path),
            write_task: None,
            path,
        });
    }

    pub fn get(cx: &App) -> &Self {
        &cx.global::<InterfaceConfigHolder>().config
    }

    pub fn force_save(cx: &mut App) {
        cx.global_mut::<InterfaceConfigHolder>().write_to_disk();
    }

    pub fn get_mut(cx: &mut App) -> &mut Self {
        // Lazy write with 5-second debounce
        if cx.global::<InterfaceConfigHolder>().write_task.is_none() {
            let task = cx.spawn(async |app| {
                app.background_executor().timer(Duration::from_secs(5)).await;
                _ = app.update_global::<InterfaceConfigHolder, _>(|holder, _| {
                    holder.write_to_disk();
                });
            });
            // ...
        }
        // ...
    }
}
```

### Configuration Updates

**Example from Settings Modal:** [crates/frontend/src/modals/settings.rs#L289-L310](crates/frontend/src/modals/settings.rs#L289-L310)

```rust
.child(Checkbox::new("hide-on-launch")
    .label(ts!("settings.windows.hide_main_window"))
    .checked(interface_config.hide_main_window_on_launch)
    .on_click(|value, _, cx| {
        InterfaceConfig::get_mut(cx).hide_main_window_on_launch = *value;
    }))
```

**Window Bounds Updates:** [crates/game_output/mod.rs#L1001-L1029](crates/frontend/src/game_output/mod.rs#L1001-L1029)

```rust
// Observe window bounds changes and save them to config
let old_window_bounds = InterfaceConfig::get(cx).game_output_bounds.clone();

// Update window bounds based on state
InterfaceConfig::get_mut(cx).game_output_bounds = new_window_bounds;

// Force save config when window is closed to ensure bounds are persisted
InterfaceConfig::force_save(cx);
```

### Write Mechanism

**Debounced Writes:** [crates/frontend/src/interface_config.rs#L236-L246](crates/frontend/src/interface_config.rs#L236-L246)

```rust
impl InterfaceConfigHolder {
    fn write_to_disk(&mut self) {
        self.write_task = None;
        let Ok(bytes) = serde_json::to_vec(&self.config) else {
            return;
        };
        _ = write_safe(&self.path, &bytes);  // Atomic write with temporary file
    }
}
```

---

## 3. PERSISTENCE MECHANISM

### `Persistent<T>` Type

**File:** [crates/backend/src/persistent.rs](crates/backend/src/persistent.rs)

Generic wrapper for persistent JSON serialization:

```rust
#[derive(Debug)]
pub struct Persistent<T: Serialize + for <'de> Deserialize<'de>> {
    path: Arc<Path>,
    dirty: bool,
    data: T
}

impl<T: Serialize + for <'de> Deserialize<'de> + Default> Persistent<T> {
    pub fn load(path: Arc<Path>) -> Self {
        let data = crate::read_json(&path).unwrap_or_default();
        Self {
            path,
            dirty: false,
            data,
        }
    }
}

impl<T: Serialize + for <'de> Deserialize<'de>> Persistent<T> {
    pub fn modify(&mut self, func: impl FnOnce(&mut T)) {
        if self.dirty {
            self.load_from_disk();
        }

        (func)(&mut self.data);

        if let Ok(bytes) = serde_json::to_vec(&self.data) {
            if crate::write_safe(&self.path, &bytes).is_ok() {
                self.dirty = true;
            }
        }
    }

    pub fn get(&mut self) -> &T {
        if self.dirty {
            self.load_from_disk();
        }
        &self.data
    }
}
```

### Atomic Write Helper

**File:** [crates/frontend/src/interface_config.rs#L252-L268](crates/frontend/src/interface_config.rs#L252-L268)

```rust
pub(crate) fn write_safe(path: &Path, content: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Write to temporary file with random suffix
    let mut temp = path.to_path_buf();
    temp.add_extension(format!("{}", rand::thread_rng().next_u32()));
    temp.add_extension("new");

    let mut temp_file = std::fs::File::create(&temp)?;
    temp_file.write_all(content)?;
    temp_file.flush()?;
    temp_file.sync_all()?;
    drop(temp_file);

    // Atomic rename
    if let Err(err) = std::fs::rename(&temp, path) {
        _ = std::fs::remove_file(&temp);
        return Err(err);
    }

    Ok(())
}
```

---

## 4. DIRECTORY STRUCTURE

**File:** [crates/backend/src/directories.rs](crates/backend/src/directories.rs)

**Configuration file locations:**

```
{launcher_root}/
├── config.json              (Backend configuration)
├── interface.json           (Frontend/UI configuration)
├── accounts.json            (Account information)
├── instances/               (Instance configurations)
├── servers/                 (Server configurations)
├── synced/                  (Synced game files)
├── metadata/                (Downloaded metadata cache)
├── assets/                  (Minecraft assets)
├── libraries/               (Java libraries)
├── temp/                    (Temporary files)
└── contentlibrary/          (Mod/content library)
```

---

## 5. CONFIGURATION INTERFACES (Frontend)

### Settings Modal

**File:** [crates/frontend/src/modals/settings.rs](crates/frontend/src/modals/settings.rs)

Two tabs:
1. **Interface Tab** - UI preferences (themes, window behavior, deletion confirmation, etc.)
2. **Network Tab** - Network settings (proxy configuration)

#### Interface Settings [crates/frontend/src/modals/settings.rs#L289-L330](crates/frontend/src/modals/settings.rs#L289-L330)
- Hide main window on launch
- Open game output after launching
- Quit all windows when main window closes
- Hide user names (privacy)
- Theme selection

#### Network Settings [crates/frontend/src/modals/settings.rs](crates/frontend/src/modals/settings.rs)
- Proxy enable/disable
- Proxy protocol (HTTP/HTTPS/SOCKS5)
- Host and port
- Authentication (username/password)

---

## 6. SUMMARY TABLE

| Aspect | Backend Config | Frontend Config |
|--------|---|---|
| **File** | `crates/schema/src/backend_config.rs` | `crates/frontend/src/interface_config.rs` |
| **Persisted File** | `config.json` | `interface.json` |
| **Format** | JSON (Serialized via serde) | JSON (Serialized via serde) |
| **Persistence Type** | `Persistent<BackendConfig>` | Global via GPUI (with debounce) |
| **Write Strategy** | On-demand via `modify()` | Debounced (5 seconds) |
| **Atomic Writes** | Yes (temporary file + rename) | Yes (write_safe helper) |
| **Settings Category** | Network/Server/Sync | UI/Interface |
| **Load Location** | Backend startup | Frontend startup |
| **Sensitive Data** | Proxy password (system keyring) | None |

---

## 7. ADDITIONAL NOTES

### Sync Targets Details
The `sync_targets` in BackendConfig allows enabling/disabling file and folder syncing across instances:

**Supported Sync Targets:**
- Files: `options.txt`, `servers.dat`, `command_history.txt`, `hotbar.nbt`
- Folders: `saves`, `config`, `screenshots`, `resourcepacks`, `shaderpacks`, `flashback`, `Distant_Horizons_server_data`, `.voxy`, `xaero`, `.bobby`, `schematics`

### Proxy Password Security
- Username is stored in `config.json`
- Password is stored in **system keyring** (via `PlatformSecretStorage`) for security
- Not persisted to disk in plaintext

### Lazy Loading & Debouncing
- **Backend**: Eager modification with immediate persistence
- **Frontend**: Lazy 5-second debounce to batch multiple setting changes

### Theme/Content Filters
- Modrinth page default: `ModrinthProjectType::Mod`
- CurseForge page default: `CurseforgeClassId::Mod`
- Configurable per user preference
