# Server Logs Handler Analysis

## Summary
The `GetServerLogFiles` handler has a path mismatch issue. It looks for logs at `server_path/.minecraft/logs` but Minecraft servers running from the root server directory create logs at `server_path/logs` instead.

---

## Handler Implementations

### 1. GetLogFiles (for Instances) - WORKING ✓
**File:** [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs#L1240-L1274)
**Lines:** 1240-1274

```rust
MessageToBackend::GetLogFiles { instance: id, channel } => {
    if let Some(instance) = self.instance_state.write().instances.get_mut(id) {
        let logs = instance.dot_minecraft_path.join("logs");

        if let Ok(read_dir) = std::fs::read_dir(logs) {
            let mut paths_with_time = Vec::new();
            let mut total_gzipped_size = 0;

            for file in read_dir {
                let Ok(entry) = file else { continue; };
                let Ok(metadata) = entry.metadata() else { continue; };
                let filename = entry.file_name();
                let Some(filename) = filename.to_str() else { continue; };

                if filename.ends_with(".log.gz") {
                    total_gzipped_size += metadata.len();
                } else if !filename.ends_with(".log") {
                    continue;
                }

                let created = metadata.created().unwrap_or(SystemTime::UNIX_EPOCH);
                let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                paths_with_time.push((Arc::from(entry.path()), created.max(modified)));
            }

            paths_with_time.sort_by_key(|(_, t)| *t);
            let paths = paths_with_time.into_iter().map(|(p, _)| p).rev().collect();
            let _ = channel.send(LogFiles { paths, total_gzipped_size: ... });
        }
    }
}
```

**How it works:**
- Gets the instance from the instance registry
- Uses `instance.dot_minecraft_path.join("logs")` - this property was set during instance creation
- Scans for `.log` and `.log.gz` files
- Sorts by modification time and returns newest first

---

### 2. GetServerLogFiles (for Servers) - BROKEN ✗
**File:** [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs#L1279-L1319)
**Lines:** 1279-1319

```rust
MessageToBackend::GetServerLogFiles { name, channel } => {
    let pandora_dir = if let Ok(dir) = std::env::var("PANDORA_DIR") {
        std::path::PathBuf::from(dir)
    } else {
        let base_dirs = directories::BaseDirs::new().unwrap();
        let data_dir = base_dirs.data_dir();
        data_dir.join("PandoraLauncher")
    };
    let servers_dir = pandora_dir.join("servers");
    let server_path = servers_dir.join(name.as_str());
    let logs = server_path.join(".minecraft/logs");  // ⚠️ WRONG PATH!

    if let Ok(read_dir) = std::fs::read_dir(logs) {
        let mut paths_with_time = Vec::new();
        let mut total_gzipped_size = 0;

        for file in read_dir {
            let Ok(entry) = file else { continue; };
            let Ok(metadata) = entry.metadata() else { continue; };
            let filename = entry.file_name();
            let Some(filename) = filename.to_str() else { continue; };

            if filename.ends_with(".log.gz") {
                total_gzipped_size += metadata.len();
            } else if !filename.ends_with(".log") {
                continue;
            }

            let created = metadata.created().unwrap_or(SystemTime::UNIX_EPOCH);
            let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            paths_with_time.push((Arc::from(entry.path()), created.max(modified)));
        }

        paths_with_time.sort_by_key(|(_, t)| *t);
        let paths = paths_with_time.into_iter().map(|(p, _)| p).rev().collect();
        let _ = channel.send(LogFiles { paths, total_gzipped_size: ... });
    }
}
```

**How it works (or doesn't):**
- Constructs path to server from PANDORA_DIR or config directory
- Looks for logs at: `servers/{server_name}/.minecraft/logs` ❌
- Silently fails when directory doesn't exist (no error handling)
- Nothing is sent back to the frontend when the read_dir fails

---

## Server Log Storage Details

### Server Directory Structure
**File:** [crates/backend/src/backend.rs](crates/backend/src/backend.rs#L1214-L1280)
**Lines:** 1214-1280 (create_server function)

When a server is created:
```
servers/
└── {server_name}/
    ├── server_metadata.json
    ├── eula.txt
    ├── server.properties
    ├── README.md
    └── plugins/  (only for Paper/Purpur)
```

**Important:** No `.minecraft` directory is created during server setup!

### Server Process Execution
**File:** [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs#L2296-L2390)
**Lines:** 2296-2390 (start_server function)

When the server starts:
```rust
let mut cmd = std::process::Command::new(&java_executable);
cmd
    .arg("-Xmx1024M")
    .arg("-Xms512M")
    .arg("-jar")
    .arg(&jar_path)
    .arg("nogui")
    .current_dir(&server_dir)  // Current working directory is server_dir (root)
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .stdin(std::process::Stdio::piped());
```

**Key Point:** The server process runs with `current_dir(&server_dir)`. When a Java Minecraft server runs, it creates:
- `logs/` directory in the current working directory (NOT `.minecraft/logs`)
- Log files like `latest.log`, `yyyy-mm-dd-1.log.gz`, etc. inside that `logs/` directory

---

## Frontend Callers

### Instance Logs (Working)
**File:** [crates/frontend/src/pages/instance/logs_subpage.rs](crates/frontend/src/pages/instance/logs_subpage.rs#L150)
```rust
self.backend_handle.send(MessageToBackend::GetLogFiles {
    instance: self.instance,
    channel: send,
});
```

### Server Logs (Broken)
**File:** [crates/frontend/src/pages/instance/server_page.rs](crates/frontend/src/pages/instance/server_page.rs#L383)
**Lines:** 383-386
```rust
self.backend_handle.send(MessageToBackend::GetServerLogFiles {
    name: self.server_name.as_str().into(),
    channel: send,
});
```

Both frontends use identical logic for handling the response, so the issue is purely in the backend path construction.

---

## Root Cause Analysis

| Aspect | GetLogFiles (Instance) | GetServerLogFiles (Server) |
|--------|----------------------|----------------------------|
| **Log Directory Path** | `instance.dot_minecraft_path.join("logs")` | `server_path.join(".minecraft/logs")` ❌ |
| **Actual Log Location** | `~/.local/share/PandoraLauncher/instances/{name}/.minecraft/logs` | `~/.local/share/PandoraLauncher/servers/{name}/logs` (no `.minecraft`) |
| **Why Path Differs** | Instances have pre-configured `dot_minecraft_path` | Servers run from root directory with Java working dir = server root |
| **Error Handling** | Silently skips if logs directory doesn't exist | Silently skips if logs directory doesn't exist |
| **User Experience** | Shows "No logs available" when no logs found | Shows "No logs available" but logs actually exist elsewhere |

---

## The Fix

Change line 1291 in [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs#L1291):

**From:**
```rust
let logs = server_path.join(".minecraft/logs");
```

**To:**
```rust
let logs = server_path.join("logs");
```

This matches where the Minecraft server actually writes logs when running from the server root directory.
