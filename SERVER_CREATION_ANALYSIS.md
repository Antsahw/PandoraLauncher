# PandoraLauncher: Server Creation Flow Analysis

## Executive Summary

The PandoraLauncher implements a complete server creation workflow with:
- **5 server software options** (Paper, Purpur, Fabric, Forge, NeoForge)
- **Modal-based UI** with version selection and customization
- **Automatic JAR downloads** via REST APIs for all server types
- **Background task architecture** for non-blocking downloads
- **Pre-configured directory structure** with all necessary files

---

## 1. UI Modal: "Create Server" Dialog

### Location & Trigger
- **Modal File:** [crates/frontend/src/modals/create_server.rs](crates/frontend/src/modals/create_server.rs)
- **Triggered From:** [crates/frontend/src/pages/servers_page.rs](crates/frontend/src/pages/servers_page.rs#L34-L39)
- **Entry Point:** Button click on Servers Page

### UI Flow
```
Servers Page
  └─ "Create Server" Button (Line 34-39)
     └─ Click handler → open_create_server()
        └─ Opens Modal Dialog with form
```

### Modal Components
The modal renders the following form fields:

1. **Server Name Input** - Text field with validation
   - Checks for uniqueness against existing servers
   - Validates filename sanitization
   - Auto-generates fallback names if needed

2. **Minecraft Version Dropdown** - Version selection
   - Populated from metadata (MinecraftVersionManifest)
   - Searchable dropdown
   - Default: Latest version
   - Shows snapshot versions if enabled in config

3. **Server Software Selector** - Button group selection
   - Paper (default)
   - Purpur
   - Fabric
   - Forge
   - NeoForge

4. **Icon Picker** - Optional server icon
   - Opens select_icon modal
   - Supports PNG format

### Modal State Structure
**File:** [crates/frontend/src/modals/create_server.rs:21-38](crates/frontend/src/modals/create_server.rs#L21-L38)

```rust
struct CreateServerModalState {
    metadata: Entity<FrontendMetadata>,           // Version manifest
    versions: Entity<FrontendMetadataState>,      // Loaded versions
    backend_handle: BackendHandle,                // Backend communication
    minecraft_version_dropdown: Entity<SelectState<VersionList>>,
    name_input_state: Entity<InputState>,
    selected_loader: ServerLoader,                // Paper/Purpur/Fabric/Forge/NeoForge
    loaded_versions: bool,
    error_loading_versions: Option<SharedString>,
    name_invalid: bool,
    server_names: Arc<[SharedString]>,
    original_fallback_name: SharedString,
    unique_fallback_name: SharedString,
    icon: Option<EmbeddedOrRaw>,
    _versions_updated_subscription: Subscription,
    _name_input_subscription: Subscription,
    _version_selected_subscription: Subscription,
}
```

### Modal Rendering
**Method:** `CreateServerModalState::render()` [Lines 205-354](crates/frontend/src/modals/create_server.rs#L205-L354)

- Uses GPUI framework
- Renders conditional skeletons while loading
- Footer buttons: Cancel & Create
- Error display with reload button if versions fail to load

---

## 2. Available Server Software Options

### Enum Definition
**File:** [crates/frontend/src/modals/create_server.rs:13-18](crates/frontend/src/modals/create_server.rs#L13-L18)

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ServerLoader {
    Paper,
    Purpur,
    Fabric,
    Forge,
    NeoForge,
}
```

### UI Rendering
**File:** [crates/frontend/src/modals/create_server.rs:253-280](crates/frontend/src/modals/create_server.rs#L253-L280)

Each option rendered as a button in a ButtonGroup:
```
[Paper] [Purpur] [Fabric] [Forge] [NeoForge]
```

Default: Paper (Line 84)

### Message Conversion
**File:** [crates/frontend/src/modals/create_server.rs:335-347](crates/frontend/src/modals/create_server.rs#L335-L347)

When user clicks "Create", the selected enum converts to string:
- `ServerLoader::Paper` → `"Paper"`
- `ServerLoader::Purpur` → `"Purpur"`
- `ServerLoader::Fabric` → `"Fabric"`
- `ServerLoader::Forge` → `"Forge"`
- `ServerLoader::NeoForge` → `"NeoForge"`

---

## 3. Backend: Message Handling & create_server Implementation

### Message Definition
**File:** [crates/bridge/src/message.rs:39-43](crates/bridge/src/message.rs#L39-L43)

```rust
CreateServer {
    name: Ustr,
    version: Ustr,
    server_software: Ustr,
    icon: Option<EmbeddedOrRaw>,
}
```

### Message Handler
**File:** [crates/backend/src/backend_handler.rs:87-92](crates/backend/src/backend_handler.rs#L87-L92)

```rust
MessageToBackend::CreateServer { name, version, server_software, icon } => {
    let clone = self.clone();
    tokio::spawn(async move {
        let _ = clone.create_server(&name, &version, &server_software, icon).await;
    });
}
```

**Key Points:**
- Handler spawns async task (non-blocking)
- Passes message parameters to `create_server()` method
- Isolated in tokio task for concurrency

### create_server Function Implementation
**File:** [crates/backend/src/backend.rs:1079-1170](crates/backend/src/backend.rs#L1079-L1170)

#### Validation Phase
```rust
// 1. Single component path check
if !crate::is_single_component_path_str(&name) {
    self.send.send_warning(...);
    return None;
}

// 2. Filename sanitization (Windows & Unix compatibility)
if !sanitize_filename::is_sanitized_with_options(...) {
    self.send.send_warning(...);
    return None;
}

// 3. Uniqueness check against existing servers
if self.instance_state.read().instances.iter().any(|i| i.name == name) {
    self.send.send_warning("Unable to create server, name is already used");
    return None;
}
```

#### Directory Creation
```rust
let server_dir = self.directories.servers_dir.join(name);
_ = std::fs::create_dir_all(&server_dir);
```

**Result:** `~/.pandora_launcher/servers/{server_name}/`

#### File Creation

**1. server_metadata.json**
```json
{
    "name": "server_name",
    "server_software": "Paper|Purpur|Fabric|Forge|NeoForge",
    "minecraft_version": "1.20.1",
    "server_port": 25565
}
```

**2. eula.txt**
```
# By changing the setting below to TRUE you are indicating your agreement to our EULA
eula=true
```
**Purpose:** Minecraft servers require EULA acceptance; auto-set to avoid manual setup

**3. server.properties** (Template)
```
# Minecraft server properties
server-port=25565
server-ip=
level-seed=
gamemode=survival
difficulty=normal
pvp=true
```

**4. plugins/ Directory**
- Created only for Paper/Purpur
- Location: `{server_dir}/plugins/`
- Ready for plugin installation

**5. README.md** (Instructions)
- Template showing how to download JAR
- Links to official sources:
  - Paper: https://papermc.io
  - Purpur: https://purpurmc.org
  - Fabric: https://fabricmc.net
  - Forge: https://minecraftforge.net
  - NeoForge: https://neoforged.net

#### Icon Handling
```rust
match icon {
    Some(EmbeddedOrRaw::Embedded(e)) => {
        let icon_metadata = server_dir.join("icon_metadata.txt");
        crate::write_safe(&icon_metadata, e.as_bytes()).unwrap();
    },
    Some(EmbeddedOrRaw::Raw(image_bytes)) => {
        if format == ImageFormat::Png {
            let icon_path = server_dir.join("server_icon.png");
            crate::write_safe(&icon_path, &*image_bytes).unwrap();
        }
    },
    None => {},
}
```

#### Background JAR Download
After all files created, spawns background task:
```rust
let sender = self.send.clone();
let server_dir_clone = server_dir.clone();
let version_str = version.to_string();
let software_str = server_software.to_string();

tokio::spawn(async move {
    let _ = download_server_jar_background(&sender, &server_dir_clone, &software_str, &version_str).await;
});
```

---

## 4. Where JAR Files Are Placed

### Directory Structure
```
launcher_dir/
└── servers/
    └── {server_name}/
        ├── server_metadata.json
        ├── eula.txt
        ├── server.properties
        ├── plugins/                    (Paper/Purpur only)
        ├── icon_metadata.txt          (if icon provided)
        ├── server_icon.png            (if PNG icon provided)
        ├── README.md
        └── {jarfile}                  (downloaded)
```

### JAR Naming Convention
By server software type:
- **Paper:** `paper-{version}.jar`
  - Example: `paper-1.20.1.jar`
- **Purpur:** `purpur-{version}.jar`
  - Example: `purpur-1.20.1.jar`
- **Fabric:** `fabric-server-{version}.jar`
  - Example: `fabric-server-1.20.1.jar`
- **Forge:** `forge-{version}-installer.jar`
  - Example: `forge-1.20.1-installer.jar`
- **NeoForge:** `neoforge-{version}-installer.jar`
  - Example: `neoforge-1.20.1-installer.jar`

### Root Launcher Directory
**File:** [crates/backend/src/directories.rs:30-35](crates/backend/src/directories.rs#L30-L35)

```rust
pub fn new(launcher_dir: PathBuf) -> Self {
    let instances_dir = launcher_dir.join("instances");
    let servers_dir = launcher_dir.join("servers");  // ← servers go here
    ...
}
```

The `launcher_dir` is determined at runtime based on:
- Platform (Windows/Linux/macOS)
- User configuration
- Typically: `~/.pandora_launcher/` or equivalent

### Filesystem Watching
When creating server, the launcher registers filesystem watch:
```rust
self.file_watching.write().watch_filesystem(
    self.directories.servers_dir.clone(),
    WatchTarget::InstancesDir
);
```

---

## 5. HTTP Client & Download Infrastructure

### HTTP Library: reqwest 0.12.24

**Dependency:** [Cargo.toml:52](Cargo.toml#L52)
```toml
reqwest = { version = "0.12.24", features = ["json", "rustls-tls", "stream", "multipart"] }
```

**Features:**
- `json` - JSON serialization/deserialization
- `rustls-tls` - Secure TLS connections
- `stream` - Streaming responses
- `multipart` - Multipart form data

### HTTP Clients in Backend
**File:** [crates/backend/src/backend.rs:30-50](crates/backend/src/backend.rs#L30-L50)

Two HTTP clients are configured:
```rust
fn build_http_clients(
    user_agent: &str,
    proxy_config: &ProxyConfig,
    proxy_password: Option<&str>,
) -> (reqwest::Client, reqwest::Client) {
    let mut http_builder = reqwest::ClientBuilder::new()
        .user_agent(user_agent)
        // ... proxy configuration
        .build();

    let mut redirecting_builder = reqwest::ClientBuilder::new()
        // ... with redirect handling
        .build();

    (http_builder, redirecting_builder)
}
```

**Stored in BackendState:**
```rust
pub http_client: reqwest::Client,
pub redirecting_http_client: reqwest::Client,
```

### Background Download Task
**File:** [crates/backend/src/backend.rs:1366-1544](crates/backend/src/backend.rs#L1366-L1544)

Async function: `async fn download_server_jar_background()`

**Architecture:**
- Creates new `reqwest::Client::new()`
- Async/await (tokio runtime)
- Non-blocking file I/O via `tokio::fs::write()`
- Progress reporting to frontend via `FrontendHandle`

### Download Endpoints by Software

#### Paper
**API:** `https://api.papermc.io/v2/projects/paper/versions/{version}`

Flow:
1. GET `/versions/{version}` → Returns JSON with build list
2. Extract latest build number from `builds` array
3. Download: `https://api.papermc.io/v2/projects/paper/versions/{version}/builds/{build}/downloads/paper-{version}-{build}.jar`
4. Save as `paper-{version}.jar`

**Example:**
```rust
let api_url = format!("https://api.papermc.io/v2/projects/paper/versions/{}", version);
let response = client.get(&api_url).send().await?;
let version_data = response.json::<serde_json::Value>().await?;
let builds = version_data["builds"].as_array()?;
let latest_build = builds.last()?.as_i64()?;
let download_url = format!(
    "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}/downloads/paper-{}-{}.jar",
    version, latest_build, version, latest_build
);
```

#### Purpur
**API:** `https://api.purpurmc.io/v2/purpur/{version}`

Flow:
1. GET `/purpur/{version}` → Returns JSON with build info
2. Extract latest build: `builds.latest`
3. Download: `https://api.purpurmc.io/v2/purpur/{version}/builds/{build}/downloads/purpur-{version}-{build}.jar`
4. Save as `purpur-{version}.jar`

#### Fabric
**API:** `https://meta.fabricmc.net/v2/versions/loader`

Flow:
1. GET `/versions/loader` → Returns array of loader versions
2. Extract first (latest) loader version
3. Download: `https://meta.fabricmc.net/v2/versions/loader/{version}/{loader}/server/jar`
4. Save as `fabric-server-{version}.jar`

**Note:** Fabric uses direct redirect to JAR (not an intermediate API call)

#### Forge
**Maven Repository:** `https://maven.minecraftforge.net/`

Flow:
1. Try: `https://maven.minecraftforge.net/net/minecraftforge/forge/{version}-latest/forge-{version}-latest-installer.jar`
2. If 404, fallback: `https://maven.minecraftforge.net/net/minecraftforge/forge/{version}/forge-{version}-installer.jar`
3. Save as `forge-{version}-installer.jar`

**Important:** Returns **installer**, not runnable JAR
- User must execute: `java -jar forge-{version}-installer.jar --installServer`

#### NeoForge
**Maven Repository:** `https://maven.neoforged.net/`

Flow:
1. GET: `https://maven.neoforged.net/releases/net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar`
2. Save as `neoforge-{version}-installer.jar`

**Important:** Returns **installer**, not runnable JAR
- User must execute: `java -jar neoforge-{version}-installer.jar --installServer`

### Error Handling & Messaging

All download operations send status messages to frontend:
```rust
sender.send_info(format!("Downloading Paper {} from API...", version));
// ... on success
sender.send_info(format!("✓ Paper server downloaded successfully"));
// ... on error
sender.send_error("Failed to download Paper JAR".to_string());
```

Messages appear in launcher UI for user feedback.

---

## 6. Complete Request Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│ Frontend: Servers Page (servers_page.rs)                        │
│  - "Create Server" button displayed                             │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     │ [User clicks]
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│ Modal Dialog (create_server.rs:CreateServerModalState)          │
│ ┌──────────────────────────────────────────────────────────┐   │
│ │ Form Inputs:                                             │   │
│ │  - Server Name [text field + uniqueness validation]      │   │
│ │  - Minecraft Version [dropdown + searchable]             │   │
│ │  - Server Software [Paper|Purpur|Fabric|Forge|NeoForge] │   │
│ │  - Icon [optional PNG image]                            │   │
│ └─────────────────────────┬──────────────────────────────┘   │
│                           │                                    │
│                           │ [User clicks Create]              │
│                           ▼                                    │
│ ┌─────────────────────────────────────────────────────────┐   │
│ │ Validation:                                             │   │
│ │  1. Name not empty                                      │   │
│ │  2. Version selected                                    │   │
│ │  3. software_software mapped to string                  │   │
│ │  4. Icon (optional) extracted                          │   │
│ └──────────────┬────────────────────────────────────────┘   │
│                │                                             │
│                │ [Create message]                           │
└────────────────┼─────────────────────────────────────────────┘
                 │
                 │ MessageToBackend::CreateServer {
                 │    name: "Survival",
                 │    version: "1.20.1",
                 │    server_software: "Paper",
                 │    icon: Some(...)
                 │ }
                 │
                 ▼
┌─────────────────────────────────────────────────────────────────┐
│ Backend Handler (backend_handler.rs:87-92)                      │
│ - Receives message                                              │
│ - Spawns tokio task                                             │
└──────────────────────┬──────────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────────┐
│ Backend::create_server() (backend.rs:1079-1170)                 │
│                                                                 │
│ ┌─────────────────────────────────────────────────────────┐   │
│ │ Phase 1: Validation                                     │   │
│ │  - Name single component path                          │   │
│ │  - Name sanitized (Windows/Unix safe)                  │   │
│ │  - Name unique vs existing instances                   │   │
│ └─┬───────────────────────────────────────────────────────┘   │
│   │                                                             │
│   ├─→ /servers/Survival/ [create directory]                   │
│   │                                                             │
│   └─→ Write files:                                             │
│       ├─ server_metadata.json                                  │
│       ├─ eula.txt (auto-set to true)                          │
│       ├─ server.properties (template)                         │
│       ├─ plugins/ (Paper/Purpur only)                         │
│       ├─ icon_metadata.txt or server_icon.png (if provided)   │
│       └─ README.md (download instructions)                    │
│                                                                 │
│ ┌─────────────────────────────────────────────────────────┐   │
│ │ Phase 2: Spawn Background Download Task                 │   │
│ │  (tokio::spawn)                                         │   │
│ └─┬───────────────────────────────────────────────────────┘   │
└───┼───────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│ Background Task: download_server_jar_background()               │
│ (backend.rs:1366-1544)                                          │
│                                                                 │
│ Based on server_software value:                                │
│                                                                 │
│ ┌─ Paper      ─────────────────────────────────────────┐      │
│ │ GET https://api.papermc.io/v2/projects/paper/...    │      │
│ │  → Fetch latest build → Download JAR                │      │
│ │  → Write: /servers/Survival/paper-1.20.1.jar        │      │
│ └──────────────────────────────────────────────────────┘      │
│                                                                 │
│ ┌─ Purpur     ─────────────────────────────────────────┐      │
│ │ GET https://api.purpurmc.io/v2/purpur/...           │      │
│ │  → Fetch latest build → Download JAR                │      │
│ │  → Write: /servers/Survival/purpur-1.20.1.jar       │      │
│ └──────────────────────────────────────────────────────┘      │
│                                                                 │
│ ┌─ Fabric     ─────────────────────────────────────────┐      │
│ │ GET https://meta.fabricmc.net/v2/versions/loader    │      │
│ │  → Get latest loader version                        │      │
│ │  → Download server JAR                              │      │
│ │  → Write: /servers/Survival/fabric-server-1.20.1.jar│      │
│ └──────────────────────────────────────────────────────┘      │
│                                                                 │
│ ┌─ Forge      ─────────────────────────────────────────┐      │
│ │ GET https://maven.minecraftforge.net/...            │      │
│ │  → Download installer JAR                           │      │
│ │  → Write: /servers/Survival/forge-1.20.1-installer │      │
│ │  → Send: "Run 'java -jar forge-...-installer.jar' " │      │
│ └──────────────────────────────────────────────────────┘      │
│                                                                 │
│ ┌─ NeoForge   ─────────────────────────────────────────┐      │
│ │ GET https://maven.neoforged.net/...                 │      │
│ │  → Download installer JAR                           │      │
│ │  → Write: /servers/Survival/neoforge-1.20.1-insta...       │
│ │  → Send: "Run 'java -jar neoforge-...-installer.jar'│      │
│ └──────────────────────────────────────────────────────┘      │
│                                                                 │
│ All operations send status messages to frontend:              │
│  - "Downloading {software} {version}..."                      │
│  - "✓ {software} downloaded successfully"                    │
│  - "Error: Failed to download..."                            │
└─────────────────────────────────────────────────────────────────┘
```

---

## Key Implementation Details

### 1. Validation Functions Used
- `crate::is_single_component_path_str()` - Prevents path traversal attacks
- `sanitize_filename::is_sanitized_with_options()` - Windows/Unix safety
- Uniqueness check against `instance_state.read().instances`

### 2. File Writing
- Custom `crate::write_safe()` function ensures safe writes
- JAR downloads use `tokio::fs::write()` for async non-blocking I/O

### 3. Async Architecture
- Modal interaction: Synchronous (GPUI event handlers)
- Directory creation: Synchronous
- JAR download: Asynchronous (tokio spawn)
- Non-blocking = UI remains responsive during download

### 4. Error Messages
- All errors/info sent via `FrontendHandle::send_*()` methods
- Display in launcher UI instantly
- Users see progress in real-time

### 5. Icon Handling
- Two formats supported: `EmbeddedOrRaw::Embedded (string)` or `EmbeddedOrRaw::Raw (bytes)`
- PNG validation for binary icons
- Stored as `server_icon.png` and `icon_metadata.txt`

### 6. Post-Creation State
After successful creation:
- Directory exists with all files ✓
- JAR download in progress (or queued) ✓
- Server listed in Servers UI ✓
- User can start server once JAR downloaded ✓

---

## Summary Table

| Aspect | Location | Details |
|--------|----------|---------|
| **Modal UI** | `crates/frontend/src/modals/create_server.rs` | GPUI form with 4 inputs |
| **Message** | `crates/bridge/src/message.rs:39-43` | `MessageToBackend::CreateServer` |
| **Handler** | `crates/backend/src/backend_handler.rs:87-92` | Spawns async task |
| **create_server()** | `crates/backend/src/backend.rs:1079-1170` | Validates + creates files |
| **Download Task** | `crates/backend/src/backend.rs:1366-1544` | Async JAR download |
| **HTTP Client** | `reqwest 0.12.24` | With json, tls, stream features |
| **Directory** | `LauncherDirectories.servers_dir` | `{launcher_dir}/servers/` |
| **Server Options** | 5 types | Paper, Purpur, Fabric, Forge, NeoForge |
| **APIs Used** | 5 different APIs | PaperMC, PurpurMC, FabricMC, MinecraftForge, NeoForge Maven repos |

