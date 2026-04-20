# PandoraLauncher Component Architecture

## Frontend Architecture Overview

```
crates/
├── backend/              # Backend server logic
│   └── src/
│       ├── backend.rs   # Main backend handler
│       ├── launch.rs    # Instance launching
│       └── log_reader.rs # Log file reading
│
├── bridge/              # Frontend-Backend communication
│   └── message types (MessageToBackend, etc.)
│
└── frontend/            # UI Layer (GPUI Framework)
    └── src/
        ├── root.rs              # Main app entry point
        ├── lib.rs               # Utilities (open_folder)
        ├── ui.rs                # UI setup
        │
        ├── game_output/         # ⭐ KILL BUTTON + FOLDER BUTTON
        │   └── mod.rs
        │       ├── GameOutput struct (log display)
        │       └── GameOutputRoot (multiple tabs, buttons)
        │           ├── kill_btn (KillInstance/StopServer messages)
        │           └── folder_btn (open with system file manager)
        │
        ├── pages/
        │   ├── instance/
        │   │   ├── instance_page.rs      # ⭐ FOLDER BUTTON (instance view)
        │   │   ├── server_page.rs        # ⭐ SERVER LOGS DROPDOWN
        │   │   ├── logs_subpage.rs       # ⭐ INSTANCE LOGS DROPDOWN
        │   │   ├── mods_subpage.rs
        │   │   ├── settings_subpage.rs
        │   │   └── server_settings_subpage.rs
        │   │
        │   ├── instances_page.rs         # Instance list
        │   ├── servers_page.rs           # Server list
        │   └── skins_page.rs             # Folder button example
        │
        ├── component/
        │   ├── named_dropdown.rs         # ⭐ DROPDOWN COMPONENT
        │   ├── readonly_text_field.rs    # ⭐ LOG DISPLAY
        │   ├── instance_list.rs
        │   └── ...
        │
        ├── entity/
        │   └── instance.rs               # Instance data structure
        │
        ├── modals/
        │   ├── create_instance.rs
        │   ├── create_server.rs
        │   └── ...
        │
        └── icon.rs                       # PandoraIcon enum
```

## Feature Interaction Diagram

### 1. SERVER LOGS DROPDOWN FLOW

```
Server Page (server_page.rs)
    │
    ├─ ServerLogsSubpage created
    │   │
    │   ├─ get_log_files() called
    │   │   │
    │   │   └─ Send: MessageToBackend::GetLogFiles
    │   │       │
    │   │       └─ Backend returns LogFiles { paths: Vec<Path> }
    │   │
    │   └─ NamedDropdown created with log file items
    │       │
    │       └─ Display: Select::new(&available_logs)
    │           │ Placeholder: "instance.logs.select_file"
    │
    └─ On dropdown selection:
       ├─ SelectEvent<NamedDropdown<Arc<Path>>>
       │
       └─ Send: MessageToBackend::ReadLog { path, channel }
           │
           └─ Backend reads and returns log content
               │
               └─ Display in ReadonlyTextFieldWithControls
                   │
                   └─ Upload button available
```

### 2. KILL BUTTON FLOW

```
Game Output View (game_output/mod.rs)
    │
    ├─ GameOutputRoot maintains state
    │   ├─ active_instance_id: Option<usize>
    │   ├─ instance_statuses: HashMap<InstanceID, InstanceStatus>
    │   ├─ server_statuses: HashMap<String, bool>
    │   └─ tabs: HashMap for multiple instances
    │
    └─ Kill Button (Lines 1550-1578)
        │
        ├─ Check instance running status
        │   └─ instance_statuses.get(instance_id) == Running
        │       │
        │       ├─ YES: on_click → MessageToBackend::KillInstance { id }
        │       │
        │       └─ NO: Check server running
        │           │
        │           ├─ YES: on_click →  MessageToBackend::StopServer { name }
        │           │       (also: server_statuses.insert(name, false))
        │           │
        │           └─ NO: DISABLED
        │
        └─ Button UI
            ├─ Icon: PandoraIcon::Close (X)
            ├─ Label: "Kill"
            ├─ Size: 24px
            └─ Color: default (or red when clickable)
```

### 3. FOLDER BUTTON FLOW

```
GAME OUTPUT VIEW (game_output/mod.rs)
    │
    └─ Folder Button (Lines 1514-1533)
        │
        ├─ Priority check:
        │   ├─ server_folders.get(&active_id)?
        │   │   └─ Use server path
        │   │
        │   └─ instance_folders.get(&active_id)?
        │       └─ Use .minecraft folder (dot_minecraft_folders)
        │
        └─ on_click:
            └─ open::that(folder_path.as_ref())
                └─ System opens folder with default file manager


INSTANCE PAGE (pages/instance/instance_page.rs)
    │
    └─ Open Folder Button (Lines 114-118)
        │
        ├─ Gets: instance.dot_minecraft_folder
        │
        └─ on_click:
            └─ crate::open_folder(&path, window, cx)
                │
                ├─ Check: path.is_dir()?
                │   └─ NO: Show error notification
                │
                └─ Call: open::that(path)
                    └─ System opens folder with default file manager


OPEN_FOLDER UTILITY (lib.rs: 359-372)
    │
    ├─ Validates path is directory
    │   └─ Show error notification if not
    │
    └─ Uses 'open' crate
        └─ Cross-platform folder opening
            ├─ Linux: xdg-open (file manager)
            ├─ Windows: explorer.exe
            └─ macOS: open command
```

## State Management

### GameOutputRoot State (Multiple Instances/Servers)

```rust
// Tracking multiple tabs
pub tabs: HashMap<usize, Entity<GameOutput>>
pub instance_names: HashMap<usize, SharedString>
pub instance_folders: HashMap<usize, Arc<Path>>
pub dot_minecraft_folders: HashMap<usize, Arc<Path>>
pub instance_ids: HashMap<usize, InstanceID>
pub instance_statuses: HashMap<InstanceID, InstanceStatus>  // Running status
pub server_names: HashMap<usize, String>
pub server_folders: HashMap<usize, Arc<Path>>
pub server_statuses: HashMap<String, bool>  // Server running state
pub active_instance_id: Option<usize>  // Currently viewed tab
```

### Log Subpage State

```rust
pub struct ServerLogsSubpage {
    server_name: SharedString,
    backend_handle: BackendHandle,
    
    // UI Components
    log_content: Option<Entity<ReadonlyTextFieldWithControls>>,
    available_logs: Option<Entity<SelectState<NamedDropdown<Arc<Path>>>>>,
    
    // State
    no_available_logs: bool,
    clean_old_logs_text: Option<SharedString>,
    
    // Subscriptions/Tasks
    _read_log_task: Option<Task<()>>,
    _dropdown_change_subscrption: Option<Subscription>,
}
```

## Message Flow: Backend Communication

```
Frontend (game_output/mod.rs or pages/instance/*)
    │
    ├─ Send: MessageToBackend enum
    │   ├─ GetLogFiles { instance, channel }
    │   ├─ ReadLog { path, send }
    │   ├─ KillInstance { id }
    │   ├─ StopServer { name }
    │   └─ ... (many others)
    │
    └─> BackendHandle.send(message)
        │
        └─> Backend (bridge or backend crate)
            │
            ├─ Process message
            │   ├─ KillInstance → signal process termination
            │   ├─ StopServer → stop server process
            │   ├─ GetLogFiles → scan for log files
            │   └─ ReadLog → read file content
            │
            └─> Return response via channel
                │
                └─> Frontend updates UI
                    ├─ dropdown items populated
                    ├─ log content displayed
                    ├─ statuses updated
                    └─ buttons enabled/disabled
```

## GPUI Component Hierarchy

```
Root App
    │
    ├── GameOutput Pane (Left side - logs)
    │   └── GameOutputRoot
    │       ├── Search Input (top)
    │       ├── Log Display (scrollable)
    │       │   └── GameOutput component (rendered log lines)
    │       │
    │       └── Bottom Action Bar
    │           ├── Server Command Input (if server)
    │           │   └── Input::new(&server_command_state)
    │           │
    │           ├── Folder Button
    │           │   └── Button::new("open_folder")
    │           │       .icon(PandoraIcon::Folder)
    │           │       .on_click(|| open::that(path))
    │           │
    │           ├── Tab Bar (multiple instances)
    │           │   └── Tab for each instance/server
    │           │
    │           └── Kill Button
    │               └── Button::new("kill_instance")
    │                   .icon(PandoraIcon::Close)
    │                   .label("Kill")
    │                   .on_click(send KillInstance/StopServer)
    │
    ├── Instance Settings Pane (Right side)
    │   ├── Tab 1: Logs
    │   │   └── InstanceLogsSubpage
    │   │       ├── Header
    │   │       │   └── Select::new(&available_logs)
    │   │       │       .placeholder("instance.logs.select_file")
    │   │       │
    │   │       └── Content
    │   │           └── ReadonlyTextFieldWithControls (log display)
    │   │
    │   ├── Tab 2: Mods
    │   │   └── InstanceModsSubpage
    │   │
    │   ├── Tab 3: Settings
    │   │   └── InstanceSettingsSubpage
    │   │
    │   └── Instance Info Bar
    │       ├── Play Button
    │       ├── Open Folder Button
    │       │   └── Button::new("open_dot_minecraft")
    │       │       .icon(PandoraIcon::FolderOpen)
    │       │       .on_click(|| crate::open_folder(path))
    │       │
    │       └── Kill Button (if running)
    │
    └── Server Page
        └── ServerLogsSubpage
            ├── Logs dropdown (empty for now)
            └── "No logs yet" message
```

## Key Dependencies

```
UI Framework:
├── gpui               # Main UI framework (from Zed)
├── gpui_platform      # Platform-specific features
└── gpui-component     # Reusable components (Button, Select, Input, etc.)

Component Libraries:
├── named_dropdown     # Custom NamedDropdown<T> component
└── readonly_text_field # Custom log display component

File System:
├── open              # Cross-platform folder/file opening
├── std::path         # Path handling
└── notify            # File system notifications (for live updates?)

Backend Bridge:
├── bridge            # Message types and communication
└── keep_alive        # Process keep-alive management

Utilities:
├── ftree             # Fenwick tree for efficient line counting
└── lru              # LRU cache for rendered lines
```

## Code Flow Examples

### Example 1: User opens server logs

```
1. User clicks Instance → Logs tab
2. ServerLogsSubpage::new() called
3. get_log_files() sends MessageToBackend::GetLogFiles
4. Backend scans logs directory, returns LogFiles { paths: [...] }
5. NamedDropdown created with received paths
6. Select component rendered with placeholder "Select log file..."
7. User clicks dropdown, selects "latest.log"
8. SelectEvent triggered
9. MessageToBackend::ReadLog sent with selected path
10. Backend reads file content, sends back
11. ReadonlyTextFieldWithControls created and rendered
12. User can now read log content and upload if needed
```

### Example 2: User kills running instance from game output

```
1. Instance is running (InstanceStatus::Running)
2. GameOutputRoot detects status
3. Kill button is enabled with on_click handler
4. User clicks Kill button
5. Button click handler executes
6. ActiveInstanceId checked → finds InstanceID
7. MessageToBackend::KillInstance { id } sent
8. Backend terminates process
9. Instance status changes to NotRunning
10. GameOutputRoot updated with new status
11. Kill button becomes disabled (grayed out)
```

### Example 3: User opens instance .minecraft folder

```
1. User clicks folder icon in game output bottom bar
2. on_click closure executes
3. Checks: server_folders → instance_folders → dot_minecraft_folders
4. Gets path: /home/user/.minecraft
5. Calls: open::that(path)
6. System default file manager opens (nautilus/dolphin/Finder/Explorer)
7. User sees instance folder contents
```
