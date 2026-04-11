# PandoraLauncher Codebase Analysis

## 1. CreateServer Message Handler & JAR Download

### Message Handler (Backend)
**File**: [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs#L87-L92)
- **Lines 87-92**: Message handler spawns async task to call `create_server()`
```rust
MessageToBackend::CreateServer { name, version, server_software, icon } => {
    let clone = self.clone();
    tokio::spawn(async move {
        let _ = clone.create_server(&name, &version, &server_software, icon).await;
    });
}
```

### Create Server Implementation
**File**: [crates/backend/src/backend.rs](crates/backend/src/backend.rs#L1077-L1170)
- **Lines 1078-1091**: Input validation (name path checks, filename sanitization, duplicate names)
- **Lines 1092-1158**: Directory and file creation:
  - Server metadata file (`server_metadata.json`)
  - EULA file (`eula.txt`)
  - Server properties template (`server.properties`)
  - Plugins directory (for Paper/Purpur)
  - README with download instructions
  - Icon handling (embedded or PNG)
- **Lines 1160-1169**: **Spawns background task for JAR download**

### JAR Download Function
**File**: [crates/backend/src/backend.rs](crates/backend/src/backend.rs#L1365-1544)

#### Paper Server Download
**Lines 1365-1393**: 
- Fetches builds from `https://api.papermc.io/v2/projects/paper/versions/{version}`
- Gets latest build number
- Downloads JAR from: `https://api.papermc.io/v2/projects/paper/versions/{version}/builds/{build}/downloads/paper-{version}-{build}.jar`
- **Error handling**: 
  - Network failure: `"Failed to download Paper JAR"`
  - Write failure: `"Failed to write JAR to disk"`

#### Purpur Server Download
**Lines 1395-1430**:
- Fetches builds from `https://api.purpurmc.io/v2/purpur/{version}`
- Gets latest build
- Downloads from: `https://api.purpurmc.io/v2/purpur/{version}/builds/{build}/downloads/purpur-{version}-{build}.jar`
- **Error handling**: Same as Paper (network/file write)

#### Fabric Server Download
**Lines 1432-1471**:
- Fetches loader from `https://meta.fabricmc.net/v2/versions/loader`
- Downloads server JAR from: `https://meta.fabricmc.net/v2/versions/loader/{version}/{latest_loader}/server/jar`
- **Error handling**: Network/file write failures

#### Forge Installer Download
**Lines 1473-1507**:
- Primary URL: `https://maven.minecraftforge.net/net/minecraftforge/forge/{version}-latest/forge-{version}-latest-installer.jar`
- Fallback URL: `https://maven.minecraftforge.net/net/minecraftforge/forge/{version}/forge-{version}-installer.jar`
- Saves as: `forge-{version}-installer.jar`
- **Error handling**: Fails over to alternate URL

#### NeoForge Installer Download
**Lines 1509-1541**:
- URL: `https://maven.neoforged.net/releases/net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar`
- **Error handling**: `"Failed to download NeoForge installer"`

### Potential JAR Download Issues
1. **File Path Issues**:
   - `tokio::fs::write(&jar_path, &jar_bytes)` can fail if directory doesn't exist (though `create_dir_all` is called beforehand)
   - Path length limits on Windows
   - Permission issues on Linux/Mac

2. **Network Issues**:
   - API server downtime
   - Invalid version numbers
   - Redirect loops or blocked requests
   - Timeout during download

3. **Storage Issues**:
   - Insufficient disk space
   - JAR file write corruption
   - File already exists but is locked

---

## 2. Instance Status & Button Updates

### Instance Status Definition
**File**: [crates/bridge/src/instance.rs](crates/bridge/src/instance.rs#L28-L30)
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstanceStatus {
    NotRunning,
    Launching,
    Running,
}
```

### Status Computation (Backend)
**File**: [crates/backend/src/instance.rs](crates/backend/src/instance.rs#L845-L853)
```rust
pub fn status(&self) -> InstanceStatus {
    if !self.processes.is_empty() {
        InstanceStatus::Running
    } else if let Some(keepalive) = &self.launch_keepalive && keepalive.is_alive() {
        InstanceStatus::Launching
    } else {
        InstanceStatus::NotRunning
    }
}
```

**Logic**:
- **Running**: If `processes` vector is not empty (child process exists)
- **Launching**: If `launch_keepalive` handle is alive (during pre-launch)
- **NotRunning**: No processes and no keepalive

### Status Propagation to Frontend
**File**: [crates/backend/src/instance.rs](crates/backend/src/instance.rs#L853-L865)
```rust
pub fn create_modify_message(&mut self) -> MessageToFrontend {
    MessageToFrontend::InstanceModified {
        id: self.id,
        name: self.name,
        icon: self.icon.clone(),
        root_path: self.resolve_real_root_path(),
        dot_minecraft_folder: self.dot_minecraft_path.clone(),
        configuration: self.configuration.get().clone(),
        status: self.status(),  // <-- Status included here
    }
}
```

**When message is sent**:
1. **Line 243**: Right after `launch_keepalive` is set (Launching state)
2. **Line 279**: After launcher finishes (becomes Running when process added)
3. **Line 309**: When keepalive drops (back to NotRunning)

### Frontend Message Processing
**File**: [crates/frontend/src/processor.rs](crates/frontend/src/processor.rs#L138-L151)

Receives `InstanceModified` message:
```rust
MessageToFrontend::InstanceModified { id, name, icon, root_path, 
                                      dot_minecraft_folder, configuration, status } => {
    // Updates instance data
    InstanceEntries::modify(&self.data.instances, id, name.as_str().into(), 
                           icon, root_path, dot_minecraft_folder, configuration, status, cx);
    
    // Notifies GameOutputRoot of status change
    if let Some(root_entity) = &self.game_output_root {
        root_entity.update(cx, |root, _| {
            root.update_instance_status(id, status);
        });
    }
}
```

---

## 3. Play/Kill Button Implementation

### Button Rendering
**File**: [crates/frontend/src/component/instance_list.rs](crates/frontend/src/component/instance_list.rs#L197-L225)

```rust
fn render_play_button(item: &InstanceEntry, index: usize, backend_handle: BackendHandle) -> Button {
    let name = item.name.clone();
    let id = item.id;
    match item.status {
        InstanceStatus::NotRunning => {
            Button::new(("start_instance", index))
                .success()  // Green
                .label(ts!("instance.start.label"))
                .on_click(move |_, window, cx| {
                    root::start_instance(id, name.clone(), None, &backend_handle, window, cx);
                })
        },
        InstanceStatus::Launching => {
            Button::new(("launching", index))
                .warning()  // Yellow/Orange
                .label("...")
        },
        InstanceStatus::Running => {
            Button::new(("kill_instance", index))
                .danger()   // Red
                .label(ts!("instance.kill"))
                .on_click({
                    let backend_handle = backend_handle.clone();
                    move |_, _, _| {
                        backend_handle.send(MessageToBackend::KillInstance { id });
                    }
                })
        },
    }
}
```

### Additional UI Implementations

**Instance Page Controls**: [crates/frontend/src/pages/instance/instance_page.rs](crates/frontend/src/pages/instance/instance_page.rs#L41-L62)
- Similar pattern: match on `instance.status` to render appropriate button
- **NotRunning**: Play button (success)
- **Launching**: Loading spinner (warning)
- **Running**: Kill + Start Again buttons (danger)

**Game Output Kill Button**: [crates/frontend/src/game_output/mod.rs](crates/frontend/src/game_output/mod.rs#L1353-L1370)
- Kill button only enabled when `InstanceStatus::Running`
- Sends `KillInstance` message when clicked

---

## 4. StartInstance Message Flow

### Message Handler
**File**: [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs#L223-L310)

**Stages**:

1. **Lines 226-241**: Create keepalive, get instance info
   - Sets `instance.launch_keepalive` (triggers Launching state)
   - Sends `MoveInstanceToTop` message
   - Sends `InstanceModified` with status=Launching
   
2. **Lines 242-255**: Get login info and run prelaunch

3. **Lines 256-269**: Launch game via launcher
   - Calls `self.launcher.launch(...)`
   
4. **Lines 270-290**: Handle launch result
   - **Success**: Add child process to `instance.processes` (triggers Running state)
   - **Error**: Set error message in modal
   
5. **Line 309**: Send final `InstanceModified` message with new status

### Status Journey
```
Initial:  NotRunning (no processes, no keepalive)
           ↓
StartInstance received
           ↓
Launch keepalive created → Launching (keepalive.is_alive() = true)
           ↓
Process spawned → Running (processes.len() > 0)
           ↓
(User kills)
           ↓
Process removed → NotRunning
```

### Kill Instance Handler
**File**: [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs#L186-L219)

```rust
MessageToBackend::KillInstance { id } => {
    // Get mutable instance
    if let Some(instance) = instance_state.instances.get_mut(id) {
        // Kill all child processes
        for mut process in instance.processes.drain(..) {
            let result = process.kill();
        }
        
        // Send modified message (status will be NotRunning now)
        self.send.send(instance.create_modify_message());
    }
}
```

---

## 5. Summary Table

| Component | File | Lines | Purpose |
|-----------|------|-------|---------|
| CreateServer Handler | backend_handler.rs | 87-92 | Routes message to create_server() |
| create_server() | backend.rs | 1077-1170 | Creates server directories and files |
| download_server_jar_background() | backend.rs | 1365-1544 | Downloads JAR for Paper/Purpur/Fabric/Forge/NeoForge |
| Instance.status() | instance.rs | 845-853 | Computes status from processes/keepalive |
| Instance.create_modify_message() | instance.rs | 853-865 | Creates MessageToFrontend with status |
| StartInstance Handler | backend_handler.rs | 223-310 | Manages launch, keepalive, and process tracking |
| KillInstance Handler | backend_handler.rs | 186-219 | Kills processes and clears process vector |
| render_play_button() | instance_list.rs | 197-225 | Renders Play/Launching/Kill button based on status |
| Processor.process() | processor.rs | 138-151 | Receives InstanceModified and updates UI |

---

## 6. Error Prevention & Issues

### JAR Download Robustness
✅ **Good**:
- Fallback URLs for Forge
- Supports multiple server software types
- Explicit error messaging for each stage

❌ **Potential Issues**:
- No retry mechanism on network failures
- No validation of downloaded JAR file integrity (no SHA1 checks visible)
- Server directory creation uses `create_dir_all` but JAR write could still fail
- No space check before download

### Status Update Mechanism
✅ **Robust**:
- Status computed fresh each time from actual process state
- Multiple checkpoints send status updates
- Both success and error paths update UI

❌ **Edge Cases**:
- No explicit "timeout" if process hangs
- Process.kill() failure isn't handled gracefully
- Rapid start/stop could cause race conditions
