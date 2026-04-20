# PandoraLauncher Feature Implementation Guide

## Overview
This guide documents where three key UI features are implemented in the PandoraLauncher codebase:
1. **Server logs/dropdown functionality**
2. **Kill button functionality in game output**
3. **Instance folder button functionality**

All frontend code is located in `crates/frontend/src/`

---

## 1. Server Logs/Dropdown Functionality

### Primary Files

#### [crates/frontend/src/pages/instance/server_page.rs](crates/frontend/src/pages/instance/server_page.rs)
**Instance Logs Dropdown (Server View)**
- **Location**: Lines 250-350
- **Component**: `ServerLogsSubpage` struct
- **Key Fields**:
  - `available_logs: Option<Entity<SelectState<NamedDropdown<Arc<Path>>>>>` - The dropdown component
  - `no_available_logs: bool` - Flag indicating no log files available
  - `log_content: Option<Entity<ReadonlyTextFieldWithControls>>` - The log content display
  
- **How it works**:
  - Creates a dropdown using `Select::new(&available_logs)` UI component
  - Loads available log files from backend via `MessageToBackend::GetLogFiles`
  - When user selects a log file from dropdown, it triggers `ReadLog` message to backend
  - Displays log content in a `ReadonlyTextFieldWithControls` component
  - Shows placeholder text: `ts!("instance.logs.select_file")`
  - Has "Clean Old Logs" button to manage log file space

#### [crates/frontend/src/pages/instance/logs_subpage.rs](crates/frontend/src/pages/instance/logs_subpage.rs)
**Instance Logs Subpage (Detailed View)**
- **Location**: Lines 1-180+
- **Component**: `InstanceLogsSubpage` struct
- **Key Fields**:
  - `available_logs: Option<Entity<SelectState<NamedDropdown<Arc<Path>>>>>` - Log files dropdown
  - `log_content: Option<Entity<ReadonlyTextFieldWithControls>>` - Log display area
  - `clean_old_logs_text: Option<SharedString>` - Shows total gzipped size of logs

- **How it works**:
  - Calls `get_log_files()` which sends `MessageToBackend::GetLogFiles` to backend
  - Backend returns `LogFiles` struct with available log file paths
  - Creates dropdown items via `NamedDropdown::create()` with file paths
  - Subscribes to dropdown change events: `SelectEvent<NamedDropdown<Arc<Path>>>`
  - When log selected, sends `MessageToBackend::ReadLog` with file path
  - Displays formatted file size (bytes, kB, MB, GB) for cleanup calculation
  - Allows uploading selected log via `root::upload_log_file()` button

#### [crates/frontend/src/component/named_dropdown.rs](crates/frontend/src/component/named_dropdown.rs)
**Dropdown Component Definition**
- Provides `NamedDropdown<T>` generic component
- Used throughout the app for file/item selection
- `NamedDropdownItem` struct contains `name: SharedString` and `item: T`

---

## 2. Kill Button Functionality in Game Output

### Primary File

#### [crates/frontend/src/game_output/mod.rs](crates/frontend/src/game_output/mod.rs)
**Game Output View with Kill Button**

- **Location**: Lines 1500-1600 (kill button implementation)
- **Main Component**: `GameOutputRoot` struct

#### Architecture:
```rust
pub struct GameOutputRoot {
    // ... other fields
    pub active_instance_id: Option<usize>,
    pub tabs: std::collections::HashMap<usize, Entity<GameOutput>>,
    pub instance_names: std::collections::HashMap<usize, SharedString>,
    pub instance_folders: std::collections::HashMap<usize, Arc<std::path::Path>>,
    public dot_minecraft_folders: std::collections::HashMap<usize, Arc<std::path::Path>>,
    instance_ids: std::collections::HashMap<usize, InstanceID>,
    instance_statuses: std::collections::HashMap<InstanceID, InstanceStatus>,
    server_names: std::collections::HashMap<usize, String>,
    server_folders: std::collections::HashMap<usize, Arc<std::path::Path>>,
    server_statuses: std::collections::HashMap<String, bool>,
    backend_handle: BackendHandle,
}
```

#### Kill Button Implementation (Lines 1536-1578):
```rust
// Create kill button
let mut kill_btn = Button::new("kill_instance")
    .icon(PandoraIcon::Close)
    .label("Kill")
    .p_1()
    .h(px(24.0));

// For running instances:
if is_instance_running {
    kill_btn = kill_btn.on_click(cx.listener(move |root, _, _, _cx| {
        root.backend_handle.send(MessageToBackend::KillInstance { id: instance_id });
    }));
}

// For running servers:
else if is_server_running {
    kill_btn = kill_btn.on_click(cx.listener(move |root, _, _, _cx| {
        // Mark server as stopped immediately
        root.server_statuses.insert(server_name.clone(), false);
        root.backend_handle.send(MessageToBackend::StopServer { name: server_name.into() });
    }));
}

// Disable if nothing is running
else {
    kill_btn = kill_btn.disabled(true);
}
```

#### Key Features:
- **Status Checking** (Lines 1540-1548):
  - Checks if instance is running: `instance_statuses.get(instance_id) == InstanceStatus::Running`
  - Checks if server is running: `server_statuses.get(server_name)`

- **Message Sending**:
  - For instances: `MessageToBackend::KillInstance { id: instance_id }`
  - For servers: `MessageToBackend::StopServer { name: server_name }`

- **UI Component**:
  - Uses `Button` from `gpui_component` library
  - Icon: `PandoraIcon::Close` (X icon)
  - Label: "Kill"
  - Height: 24px
  - Padding: 1 unit on all sides
  - Disabled when no process running

#### Tabbed Mode Support:
- Multiple tabs for multiple instances/servers
- `active_instance_id: Option<usize>` tracks which tab is active
- All status and folder info stored in HashMaps keyed by output_id
- Bottom bar with action buttons shared across all tabs

---

## 3. Instance Folder Button Functionality

### Primary Files

#### [crates/frontend/src/game_output/mod.rs](crates/frontend/src/game_output/mod.rs) (Folder Button in Game Output)
**Location**: Lines 1514-1533
- **Component**: Folder button in the bottom action bar of game output

```rust
// Add folder button - check both instance and server folders
let folder_path = if let Some(server_path) = self.server_folders.get(&active_id) {
    Some(server_path.clone())
} else if let Some(_instance_path) = self.instance_folders.get(&active_id) {
    // For instances, open the .minecraft folder instead of the root
    self.dot_minecraft_folders.get(&active_id).cloned()
} else {
    None
};

if let Some(folder_path) = folder_path {
    let folder_btn = Button::new("open_folder")
        .icon(PandoraIcon::Folder)
        .on_click(cx.listener(move |_, _, _, _| {
            let _ = open::that(folder_path.as_ref());
        }))
        .p_1()
        .h(px(24.0));
    
    action_buttons = action_buttons.child(folder_btn);
}
```

#### [crates/frontend/src/pages/instance/instance_page.rs](crates/frontend/src/pages/instance/instance_page.rs)
**Instance Page Open Folder Button**
- **Location**: Lines 100-120
- **Component**: "Open Folder" button in instance detail view

```rust
let open_dot_minecraft_button = Button::new("open_dot_minecraft")
    .info()
    .icon(PandoraIcon::FolderOpen)
    .label(ts!("instance.open_folder"))
    .on_click({
        let dot_minecraft = instance.dot_minecraft_folder.clone();
        move |_, window, cx| {
            crate::open_folder(&dot_minecraft, window, cx);
        }
    });
```

#### [crates/frontend/src/lib.rs](crates/frontend/src/lib.rs)
**Open Folder Utility Function**
- **Location**: Lines 359-372
- **Function**: `open_folder(path: &Path, window: &mut Window, cx: &mut App)`

```rust
pub(crate) fn open_folder(path: &Path, window: &mut Window, cx: &mut App) {
    if !path.is_dir() {
        // Handle error: not a directory
        let notification: Notification = (...).into();
        window.push_notification(notification, cx);
    }
    
    // Use 'open' crate to open folder with system default file manager
    if let Err(err) = open::that(path) {
        // Handle open error
        window.push_notification(notification, cx);
    }
}
```

#### [crates/frontend/src/pages/instance/skins_page.rs](crates/frontend/src/pages/instance/skins_page.rs)
**Alternative Folder Navigation - Skins**
- **Location**: Lines 466-475
- Similar implementation for opening skins folder
- Uses same `crate::open_folder()` utility function

#### Data Storage (game_output/mod.rs):
```rust
pub instance_folders: std::collections::HashMap<usize, Arc<std::path::Path>>,
pub dot_minecraft_folders: std::collections::HashMap<usize, Arc<std::path::Path>>,
pub server_folders: std::collections::HashMap<usize, Arc<std::path::Path>>,
```

#### Navigation Logic:
1. **Priority order for game output view**:
   - First tries to open server folder if available
   - Falls back to .minecraft folder for instances
   - Uses system default file manager via `open::that()` crate

2. **Instance page**:
   - Opens `.minecraft` folder directly
   - Located at `instance.dot_minecraft_folder`
   - Shows folder open icon animation

3. **Error Handling**:
   - Validates path is a directory
   - Displays error notification if folder can't be opened
   - Uses localized error messages via `ts!()` macro

---

## Related Components

### UI Framework
- **Main**: GPUI (from Zed Industries)
- **Component Library**: `gpui-component` (provides Button, Select, Input, etc.)
- **Icon Library**: `PandoraIcon` custom enum for application icons

### Message Bridge
- **Backend Communication**: `MessageToBackend` enum
- **Key messages used**:
  - `GetLogFiles { instance: InstanceID, channel: Sender }`
  - `ReadLog { path: Arc<Path>, send: Sender }`
  - `KillInstance { id: InstanceID }`
  - `StopServer { name: String }`

### Supporting Components
- `NamedDropdown<T>` - Generic dropdown component with labeled items
- `ReadonlyTextFieldWithControls` - Log content display with buttons
- `SelectState<T>` - State management for dropdown selection

---

## File Structure Summary

```
crates/frontend/src/
├── game_output/
│   └── mod.rs                          # Kill button + folder button (game output view)
├── pages/instance/
│   ├── instance_page.rs               # Instance folder button
│   ├── server_page.rs                 # Server logs dropdown
│   ├── logs_subpage.rs                # Instance logs dropdown/display
│   └── skins_page.rs                  # Alternative folder button example
├── component/
│   ├── named_dropdown.rs              # Dropdown component definition
│   └── readonly_text_field.rs         # Log display component
├── lib.rs                              # open_folder() utility function
└── icon.rs                             # PandoraIcon definitions
```

---

## Key Takeaways

| Feature | Main File | Component | Key Message |
|---------|-----------|-----------|------------|
| Server Logs Dropdown | `pages/instance/server_page.rs` | `ServerLogsSubpage` | `GetLogFiles` / `ReadLog` |
| Kill Button | `game_output/mod.rs` | `GameOutputRoot` | `KillInstance` / `StopServer` |
| Folder Navigation | `game_output/mod.rs` + `instance_page.rs` | `GameOutputRoot` / `InstancePage` | Uses `open::that()` crate |

All features are implemented using GPUI's reactive UI framework with persistent state management and async backend communication.
