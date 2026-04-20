# Frontend Server List Display and Refresh - Analysis

## Overview
This document details how the server list is displayed in the UI, how the `ServerAdded` message is handled, and what mechanisms trigger server list refreshes when new servers are created.

---

## 1. Server List Display Components

### Main Server List Page
**File:** [crates/frontend/src/pages/servers_page.rs](crates/frontend/src/pages/servers_page.rs)

```rust
pub struct ServersPage {
    server_table: Entity<TableState<ServerList>>,
    view_dropdown: Entity<SelectState<NamedDropdown<ServersViewMode>>>,
    metadata: Entity<FrontendMetadata>,
    backend_handle: BackendHandle,
}
```

**Key Features:**
- Creates a server list table via `ServerList::create_table()`
- Supports two view modes: **Cards** and **List** (controlled by `view_dropdown`)
- Located at page route: `PageType::Servers`

### Server List Component
**File:** [crates/frontend/src/component/server_list.rs](crates/frontend/src/component/server_list.rs)

**Data Structure:**
```rust
#[derive(Clone, Debug)]
pub struct ServerEntry {
    pub name: SharedString,
    pub software: SharedString,
    pub version: SharedString,
    pub path: PathBuf,
    pub status: ServerStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running,
}

pub struct ServerList {
    pub items: Vec<ServerEntry>,
    backend_handle: BackendHandle,
}
```

**Table Structure:**
- **Columns:** 4 columns
  - Col 0: Controls (Start/Stop buttons) - 150px fixed
  - Col 1: Server Name - 150px, sortable
  - Col 2: Software - 100px (Paper, Purpur, etc.)
  - Col 3: Version - 120px, sortable

---

## 2. Server List Initialization and Data Loading

### Initial Load - File System Based
**Location:** [crates/frontend/src/component/server_list.rs](crates/frontend/src/component/server_list.rs#L38)

```rust
pub fn create_table(data: &crate::entity::DataEntities, window: &mut Window, cx: &mut App) 
    -> Entity<gpui_component::table::TableState<Self>> {
    // Reads from disk:
    let pandora_dir = std::env::var("PANDORA_DIR")
        .unwrap_or_else(|_| {
            let base_dirs = directories::BaseDirs::new().unwrap();
            base_dirs.data_dir().join("PandoraLauncher")
        });
        
    let servers_dir = pandora_dir.join("servers");
    
    // Scans all directories in servers_dir
    for entry in servers_dir.read_dir() {
        let metadata_path = path.join("server_metadata.json");
        if let Ok(content) = fs::read_to_string(&metadata_path) {
            // Parse JSON to extract: name, server_software, minecraft_version
            items.push(ServerEntry { ... });
        }
    }
    
    // Creates TableState with these items
    cx.new(|cx| {
        TableState::new(server_list, window, cx)
    })
}
```

**Key Point:** Server data is loaded **once when the page is created** by reading from the file system. The list is not dynamically subscribed to updates.

---

## 3. ServerAdded Message Handling

### Message Definition
**File:** [crates/bridge/src/message.rs](crates/bridge/src/message.rs#L307)

```rust
pub enum MessageToFrontend {
    // ... other variants ...
    ServerAdded {
        name: Ustr,
        software: Ustr,
        version: Ustr,
        path: Arc<Path>,
    },
    ServerUpdated {
        name: Ustr,
        software: Ustr,
        version: Ustr,
        path: Arc<Path>,
    },
}
```

### Message Processing
**File:** [crates/frontend/src/processor.rs](crates/frontend/src/processor.rs#L364)

```rust
pub fn process(&mut self, message: MessageToFrontend, cx: &mut App) {
    match message {
        // ... other message types ...
        MessageToFrontend::ServerAdded { .. } | MessageToFrontend::ServerUpdated { .. } => {
            // Server was added or updated - trigger UI refresh
            if let Some(handle) = self.main_window_handle {
                _ = handle.update(cx, |_, window, _cx| {
                    window.refresh();  // ← KEY CALL
                });
            }
        }
    }
}
```

**What Happens:**
1. Backend sends `ServerAdded` message when server is created
2. Processor receives it and calls `window.refresh()`
3. This triggers a UI re-render of the entire window

---

## 4. UI Refresh Mechanism

### How window.refresh() Works
- Calls `window.refresh()` which is a GPUI (GUI framework) method
- Causes the entire window to re-render by invoking `Render` implementations
- When ServersPage re-renders, it calls the render method again

### ServersPage Render Flow
**Location:** [crates/frontend/src/pages/servers_page.rs](crates/frontend/src/pages/servers_page.rs#L77)

```rust
impl Render for ServersPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match InterfaceConfig::get(cx).servers_view_mode {
            ServersViewMode::Cards => {
                let cards = self.server_table.update(cx, |table, cx| {
                    let rows = table.delegate().rows_count(cx);
                    // Render each card
                    (0..rows).map(|i| table.delegate().render_card(i, cx))
                        .collect::<Vec<_>>()
                });
                div().p_4().child(ResponsiveGrid::new(size)...)
            },
            ServersViewMode::List => {
                DataTable::new(&self.server_table).bordered(false)
            },
        }
    }
}
```

**Problem:** The server list is **not automatically reloaded** from disk during re-render. The `server_table` entity still contains the old data.

---

## 5. Server Creation Modal

### Create Server Dialog
**File:** [crates/frontend/src/modals/create_server.rs](crates/frontend/src/modals/create_server.rs)

**Key State:**
```rust
struct CreateServerModalState {
    metadata: Entity<FrontendMetadata>,
    versions: Entity<FrontendMetadataState>,
    backend_handle: BackendHandle,
    minecraft_version_dropdown: Entity<SelectState<VersionList>>,
    name_input_state: Entity<InputState>,
    selected_loader: ServerLoader,  // Paper, Purpur, Fabric, Forge, NeoForge
    loaded_versions: bool,
    name_invalid: bool,
    server_names: Arc<[SharedString]>,  // Existing server names
    _versions_updated_subscription: Subscription,
    _name_input_subscription: Subscription,
    _version_selected_subscription: Subscription,
}
```

**Subscriptions:**
1. `_version_selected_subscription` - Updates fallback name when version changes
2. `_name_input_subscription` - Validates name uniqueness against `server_names`
3. `_versions_updated_subscription` - Reloads dropdown when version manifest updates

**Creating a Server:**
- User enters server name and selects software + version
- Calls `backend_handle.send(MessageToBackend::CreateServer { ... })`
- Backend creates folder and metadata file
- Backend sends `MessageToFrontend::ServerAdded` back to frontend
- Frontend calls `window.refresh()` (as shown in section 3)

---

## 6. Server List Rendering

### Card View Rendering
**Location:** [crates/frontend/src/component/server_list.rs](crates/frontend/src/component/server_list.rs#L85)

```rust
pub fn render_card(&self, index: usize, cx: &mut App) -> Div {
    let item = &self.items[index];  // Displays existing item data
    let software_and_version = format!("{} {}", item.software, item.version);
    
    // Renders based on current status
    let status_button = match item.status {
        ServerStatus::Stopped => Button::new(...) 
            .on_click(|_, _, _| {
                backend_handle.send(MessageToBackend::StartServer { ... })
            }),
        ServerStatus::Starting => Button::new("Starting..."),
        ServerStatus::Running => Button::new(...) 
            .on_click(|_, _, _| {
                backend_handle.send(MessageToBackend::StopServer { ... })
            }),
    };
    
    // Returns card UI with buttons and metadata
}
```

### Table View Rendering
**Location:** [crates/frontend/src/component/server_list.rs](crates/frontend/src/component/server_list.rs#L225)

```rust
impl TableDelegate for ServerList {
    fn render_td(&mut self, row_ix: usize, col_ix: usize, ...) -> impl IntoElement {
        let item = &mut self.items[row_ix];
        
        match col_ix {
            0 => { /* Render Start/Stop/View buttons */ },
            1 => div().child(item.name.clone()),
            2 => div().child(item.software.clone()),
            3 => div().child(item.version.clone()),
            _ => div(),
        }
    }
}
```

---

## 7. Current Issues and Limitations

### Issue 1: Server List Not Dynamically Updated
**Problem:** When `ServerAdded` is received:
1. `window.refresh()` is called
2. ServersPage.render() is re-invoked
3. But `self.server_table` still contains the old items list
4. The new server doesn't appear without manual page refresh or reload

**Root Cause:** 
- `create_table()` is called only once during ServersPage construction
- Server data is only read from disk at that time
- No subscription/observer listens to ServerAdded to update the items list

### Issue 2: No UI State Observers for Server Changes
Unlike instances which have:
```rust
pub struct LauncherUI {
    _instance_added_subscription: Subscription,
    _instance_modified_subscription: Subscription,
    _instance_removed_subscription: Subscription,
    _instance_moved_to_top_subscription: Subscription,
}
```

Servers have **no equivalent subscriptions** at the UI level to react to ServerAdded/ServerUpdated messages.

---

## 8. Data Flow Diagram

```
User Creates Server (UI Modal)
    ↓
MessageToBackend::CreateServer sent
    ↓
Backend validates and creates folder
    ↓
Backend sends MessageToFrontend::ServerAdded
    ↓
Processor::process() receives message
    ↓
window.refresh() called
    ↓
ServersPage::render() invoked
    ↓
Renders using OLD self.server_table.items ← PROBLEM: Not updated!
    ↓
New server NOT visible in list
```

---

## 9. Recommended Solutions

### Option A: Reload From Disk During Render
Modify ServersPage to reload server list from disk on each render after ServerAdded notification.

### Option B: Add Server Subscriptions
Similar to instances, add subscriptions to LauncherUI that listen to ServerAdded/Updated messages and update the table state directly.

### Option C: Entity-Based State Management
Create a ServerList entity that manages server state and subscribes to ServerAdded messages, updating items reactively.

### Option D: Manual Refresh Trigger
Add a "Refresh" button to servers page that calls `ServerList::create_table()` again.

---

## 10. Key Files Summary

| File | Purpose |
|------|---------|
| [crates/frontend/src/pages/servers_page.rs](crates/frontend/src/pages/servers_page.rs) | Main servers page component |
| [crates/frontend/src/component/server_list.rs](crates/frontend/src/component/server_list.rs) | Server list table & rendering |
| [crates/frontend/src/modals/create_server.rs](crates/frontend/src/modals/create_server.rs) | Create server modal dialog |
| [crates/frontend/src/processor.rs](crates/frontend/src/processor.rs) | Message processor, handles ServerAdded |
| [crates/bridge/src/message.rs](crates/bridge/src/message.rs) | Message definitions |
| [crates/frontend/src/ui.rs](crates/frontend/src/ui.rs) | Main UI root with subscriptions |
