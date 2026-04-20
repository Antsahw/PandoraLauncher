# PandoraLauncher Server Architecture & Management

## Quick Summary
- **Servers are persistent** (stored in `.../servers/` directories)
- **No ServerConfiguration** - servers use flat metadata files, not structured configs like instances
- **Server data** is stored on disk + loaded from Minecraft's `servers.dat`
- **Message-driven** architecture with dedicated server operation messages
- **Backend-managed** via filesystem operations plus JAR download tasks

---

## 1. Are Servers Persistent or Ephemeral?

### Answer: **PERSISTENT**
- Servers are created as permanent directories on disk: `~/.local/share/PandoraLauncher/servers/{SERVER_NAME}/`
- Each server directory contains:
  - `server_metadata.json` - launcher-specific metadata (software type, version, port)
  - `eula.txt` - Minecraft EULA acceptance flag
  - `server.properties` - Minecraft server config
  - `README.md` - Setup instructions
  - `plugins/` directory - Optional, created for Paper/Purpur servers
  - `server_icon.png` - Optional server icon (if provided at creation)
  - `server.jar` (or format-specific jar) - The actual server executable

- Once created, servers persist indefinitely until manually deleted
- Server state is managed by the Launcher, not ephemeral

---

## 2. Server Data Storage & Management

### **File-System Based Storage**

**Location**: `~/.local/share/PandoraLauncher/servers/`
- Base: Determined by `PANDORA_DIR` env var or `BaseDirs::data_dir()/PandoraLauncher`
- Each server: `servers/{name}/`

**Server Metadata** (`server_metadata.json`):
```json
{
  "server_software": "Paper|Purpur|Fabric|Forge|NeoForge",
  "version": "1.20.1",
  "server_port": 25565
}
```

**Server Configuration** (`.minecraft/server_config.json`):
```rust
pub struct ServerConfiguration {
    pub java: Option<ServerJavaConfiguration>,
}

pub struct ServerJavaConfiguration {
    pub enabled: bool,
    pub runtime_name: String,  // "java21", "system", etc.
    pub memory: Option<ServerMemoryConfiguration>,
}

pub struct ServerMemoryConfiguration {
    pub min: u32,  // Min heap in MB (default 512)
    pub max: u32,  // Max heap in MB (default 1024)
}
```

**Instance Server List** (`servers.dat`):
- Located at: `instance/.minecraft/servers.dat`
- **Purpose**: This is Minecraft's native server list file (binary format)
- **Loaded by**: `Instance::load_servers()` function
- **Used for**: Displaying servers when player opens Multiplayer menu in Minecraft

**Server Summary Data** (from `InstanceServerSummary`):
```rust
pub struct InstanceServerSummary {
    pub name: Arc<str>,        // Server display name
    pub ip: Arc<str>,          // IP address or hostname
    pub png_icon: Option<Arc<[u8]>>,  // Server icon (PNG bytes)
}
```

---

## 3. Server Configuration Structure

### **Answer: NO - No Equivalent to InstanceConfiguration**

Servers have a **different architecture** than instances:

| Aspect | Instance | Server |
|--------|----------|--------|
| **Config File** | `info_v1.json` (structured JSON) | `server_metadata.json` + `.minecraft/server_config.json` |
| **Type** | `InstanceConfiguration` struct | Two separate JSON files |
| **What it stores** | Minecraft version, loader, JVM settings, etc. | Minimal metadata + Java runtime config |
| **Java Settings** | Part of main config | Separate `ServerConfiguration` struct |
| **Persistence** | Persistent object in code | Direct filesystem storage |

**Instance Configuration** (`InstanceConfiguration`):
- Highly structured with many settings
- Equivalent location: `instances/{id}/info_v1.json`

**Server Configuration** (simpler):
- Metadata + Java runtime config only
- Split across two files:
  1. `servers/{name}/server_metadata.json` (software, version, port)
  2. `servers/{name}/.minecraft/server_config.json` (Java runtime)

### Why Different?
- **Instances**: Launcher-managed Minecraft client environments (complex)
- **Servers**: User-provided server JARs (simple metadata tracking)

---

## 4. Server Creation / Deletion / Management

### **Backend Operations** (in `crates/backend/src/backend_handler.rs`)

#### **CreateServer** Message Handler
```rust
MessageToBackend::CreateServer { 
    name, version, server_software, icon, modal_action 
} => {
    // Spawns async task
    clone.create_server(&name, &version, &server_software, icon, &modal_action_clone).await
}
```

**Implementation** (`backend.rs` lines 1140-1244):
1. **Validation**:
   - Check name is not a path
   - Check name is valid
   - Check name not already used

2. **Directory Creation**:
   - `servers_dir.join(name)` → creates server directory
   - Sets up file watching on servers directory

3. **Metadata File Creation**:
   ```rust
   let server_metadata = json!({
       "version": version,
       "server_software": server_software,
       "server_port": 25565,
   });
   let metadata_path = server_dir.join("server_metadata.json");
   ```

4. **Setup Files**:
   - `eula.txt` → "eula=false" (Minecraft requires acceptance)
   - `server.properties` → Template with basic settings
   - `plugins/` → Directory for Paper/Purpur plugins
   - `README.md` → Instructions for setup

5. **Icon Handling** (if provided):
   - Converts embedded icon to PNG
   - Saves as `server_icon.png`
   - Writes metadata to `icon_metadata.txt`

6. **Asynchronous JAR Download**:
   - Calls `download_server_jar_background()` in background task
   - Downloads appropriate server JAR based on software type
   - Sends progress updates via modal tracker
   - **Results in**:
     ```rust
     sender.send(MessageToFrontend::ServerAdded {
         path: Arc::from(server_dir.to_path_buf()),
     });
     ```

#### **DeleteServer** Message Handler
```rust
MessageToBackend::DeleteServer { name } => {
    let servers_dir = pandora_dir.join("servers");
    let server_path = servers_dir.join(name.as_str());
    std::fs::remove_dir_all(&server_path)?;
    send(MessageToFrontend::Refresh);
}
```
- Recursively deletes entire server directory
- Sends `Refresh` message to UI
- Returns success/error message

#### **RenameServer** Message Handler
```rust
MessageToBackend::RenameServer { old_name, new_name } => {
    let old_path = servers_dir.join(old_name.as_str());
    let new_path = servers_dir.join(new_name.as_str());
    std::fs::rename(&old_path, &new_path)?;
}
```
- Simple filesystem rename
- Validates new name doesn't exist
- Sends `Refresh` message to UI

#### **StartServer** Message Handler
```rust
pub async fn start_server(&self, name: &str, modal_action: &ModalAction) {
    // 1. Load server config
    let server_config_path = server_dir.join(".minecraft").join("server_config.json");
    let server_runtime_name = load_and_parse_server_config(&server_config_path);
    
    // 2. Find server JAR file (searches for *.jar)
    let jar_file = find_jar_in_directory(&server_dir);
    
    // 3. Prepare JVM arguments with configured memory
    let jvm_args = build_jvm_args(&server_runtime_name, &memory_config);
    
    // 4. Spawn process
    let mut child = Command::new("java")
        .args(&jvm_args)
        .arg("-jar")
        .arg(&jar_file)
        .arg("nogui")
        .current_dir(&server_dir)
        .spawn()?;
    
    // 5. Store process handle
    self.server_processes.write().insert(name.to_string(), child);
    
    // 6. Set up game output logging
    let game_output_id = register_game_output(&child.stdout);
    self.server_game_output_ids.write().insert(name.to_string(), game_output_id);
}
```

**Key Details**:
- Reads `server_config.json` for Java runtime and memory settings
- **Default memory**: 512 MB min, 1024 MB max
- Spawns server process with `java -jar server.jar nogui`
- Attaches stdout/stderr to game output logging
- Stores process handle for later termination

#### **StopServer** Message Handler
```rust
pub async fn stop_server(&self, name: &str) {
    if let Some(mut child) = self.server_processes.write().remove(name) {
        child.kill()?;
        send_info("Server stopped");
    }
}
```
- Retrieves stored process from map
- Calls `kill()` to terminate
- Removes from process tracking

#### **SendServerCommand** (Stub)
```rust
MessageToBackend::SendServerCommand { name, command } => {
    // TODO: Write to stdin for command execution
}
```
- Currently not implemented
- Would write commands to server stdin

#### **ReadServerFile** / **WriteServerFile**
- Allow reading/writing arbitrary files in server directory
- Used for editing `server.properties`, `server.txt`, etc.
- Validation ensures files are within server directory

---

## 5. Server-Related Message Types

### **Messages TO Backend** (from Frontend)
```rust
pub enum MessageToBackend {
    // Server operations
    CreateServer {
        name: Ustr,
        version: Ustr,
        server_software: Ustr,  // "Paper", "Purpur", "Fabric", etc.
        icon: Option<EmbeddedOrRaw>,
        modal_action: ModalAction,
    },
    
    StartServer {
        name: Ustr,
        modal_action: ModalAction,
    },
    
    StopServer {
        name: Ustr,
    },
    
    DeleteServer {
        name: Ustr,
    },
    
    RenameServer {
        old_name: Ustr,
        new_name: Ustr,
    },
    
    SendServerCommand {
        name: Ustr,
        command: Ustr,
    },
    
    ReadServerFile {
        name: Ustr,
        filename: Ustr,
    },
    
    WriteServerFile {
        name: Ustr,
        filename: Ustr,
        content: Arc<str>,
    },
    
    // Server discovery
    RequestLoadServers {
        id: InstanceID,  // Which instance to load servers for
    },
}
```

### **Messages FROM Backend** (to Frontend)
```rust
pub enum MessageToFrontend {
    // Server list updates
    InstanceServersUpdated {
        id: InstanceID,
        servers: Arc<[InstanceServerSummary]>,
    },
    
    ServerAdded {
        path: Arc<Path>,
    },
    
    ServerUpdated {
        path: Arc<Path>,
        summary: InstanceServerSummary,
    },
    
    // File operations
    ServerFileContent {
        filename: Ustr,
        content: Arc<str>,
    },
    
    // Generic
    Refresh,
}
```

---

## 6. Server Process Management

### **Process Tracking** (in `BackendState`)
```rust
pub struct BackendState {
    // Server process storage
    pub server_processes: Arc<RwLock<HashMap<String, std::process::Child>>>,
    pub server_game_output_ids: Arc<RwLock<HashMap<String, usize>>>,
}
```

- **`server_processes`**: Map of running server processes (keyed by server name)
- **`server_game_output_ids`**: Map of game output stream IDs (for logging)

### **Server File Handling**
- Servers are **not** managed by instances
- **Independent directory** structure
- **No integration** with instance worlds/saves (no linking)

---

## 7. Server Loaders Supported

**Frontend UI** (`create_server.rs`):
```rust
pub enum ServerLoader {
    Paper,       // Spigot-based
    Purpur,      // Paper variant
    Fabric,      // Modding framework
    Forge,       // Modding framework
    NeoForge,    // Forge successor
}
```

**Installation Flow**:
1. **Paper/Purpur**: Direct JAR download from Maven repos
2. **Fabric**: Download vanilla JAR + fabric-installer, run installer
3. **Forge**: Download vanilla JAR + forge installer, run installer
4. **NeoForge**: Similar to Forge

---

## 8. Key Architectural Differences: Servers vs Instances

| Aspect | Instances | Servers |
|--------|-----------|---------|
| **Storage** | `instances/` directory tree | `servers/` flat directories |
| **Config** | `info_v1.json` (structured) | Metadata JSON + server config |
| **Managed By** | `Instance` struct (persistent object) | Filesystem + direct operations |
| **Launching** | Via launcher (JVM setup, mods, etc.) | Direct JAR execution |
| **Dependencies** | Can be complex (mods, versions) | Just software + version |
| **Updating** | Through launcher's content system | Manual or external tools |
| **Backups** | Save worlds in `.minecraft/saves/` | Standard Minecraft backups |
| **Processes** | Tracked separately per instance | Tracked in `server_processes` map |
| **Message Types** | Many (InstanceAdded, InstanceUpdated, etc.) | Fewer (ServerAdded, InstanceServersUpdated) |

---

## 9. Server Lifecycle Example

```
1. CreateServer { name: "MyServer", version: "1.20.1", software: "Paper" }
   ↓
2. Backend: Create directory structure
   - Create: ~/.../servers/MyServer/
   - Write: server_metadata.json, eula.txt, server.properties
   ↓
3. Backend: Spawn async JAR download
   - Download Paper 1.20.1 JAR
   - Place in: ~/.../servers/MyServer/paper-1.20.1.jar
   ↓
4. Backend: Send MessageToFrontend::ServerAdded
   ↓
5. Frontend: Display new server in UI
   ↓
6. User clicks "Start Server"
   ↓
7. StartServer { name: "MyServer", ... }
   ↓
8. Backend: 
   - Load server_config.json (Java settings)
   - Find paper-1.20.1.jar
   - Spawn: java -Xms512m -Xmx1024m -jar paper-1.20.1.jar nogui
   - Store process handle
   ↓
9. Server runs, outputs logged
   ↓
10. User clicks "Stop Server"
    ↓
11. Backend: Calls process.kill()
    ↓
12. Server terminates
```

---

## 10. Critical Implementation Notes

### **No Instance-Server Linking**
- Servers are completely separate from instances
- No automatic server discovery per instance
- Servers are **global** in the launcher, not per-instance

### **Static Server List Loading**
- Servers loaded from filesystem directory scan (not reactive)
- **Issue**: New servers not visible until tab refresh
- See `server_list.rs` frontend loading logic

### **JAR Download Happens Async**
- `download_server_jar_background()` is a spawned task
- Uses `ProgressTracker` for UI updates
- **Problem**: File watching on servers directory may not catch all changes

### **Server Configuration is Optional**
- `server_config.json` is **optional** (created on first start)
- Allows per-server Java runtime overrides
- Falls back to global defaults if not present

### **Minecraft's native servers.dat**
- Stored in instance's `.minecraft/servers.dat` (not in launcher servers/)
- Used by actual Minecraft client
- Separate from launcher's server management

