# PandoraLauncher Codebase Exploration - Statistics Tab Implementation Guide

## Executive Summary

The PandoraLauncher codebase is well-structured with a **backend/frontend bridge architecture**. Time tracking data (playtime) is already implemented and stored in the instance data structures. A new Statistics tab can be added by following the existing subpage pattern.

---

## 1. TAB IMPLEMENTATION STRUCTURE

### Current Implementation

**Location:** [crates/frontend/src/pages/instance/instance_page.rs](crates/frontend/src/pages/instance/instance_page.rs#L153-L175)

The instance view currently has **5 tabs**:
1. **Quickplay** (Index 0) - Browse worlds and servers
2. **Logs** (Index 1) - View instance logs
3. **Mods** (Index 2) - Manage mods
4. **Resource Packs** (Index 3) - Manage resource packs
5. **Settings** (Index 4) - Instance configuration

### Tab Architecture Pattern

**State Management:**
- Each tab is represented by an `InstanceSubpageType` enum variant
- Tab selection persists in `InterfaceConfig::instance_subpage`
- Configuration automatically saves to disk at `~/.config/PandoraLauncher/interface_config.json`

**UI Rendering:**
- Uses GPUI's `TabBar` component with `.underline()` style
- Each tab is a `Tab::new().label(ts!("translation.key"))`
- Tab index maps to `InstanceSubpageType` enum variants

**Example from instance_page.rs:**
```rust
TabBar::new("bar")
    .selected_index(selected_index)
    .underline()
    .child(Tab::new().label(ts!("instance.quickplay")))
    .child(Tab::new().label(ts!("instance.logs.title")))
    .child(Tab::new().label(ts!("instance.content.mods")))
    .child(Tab::new().label(ts!("instance.content.resourcepacks")))
    .child(Tab::new().label(ts!("settings.title")))
    .on_click(cx.listener(|_, index, _, cx| {
        let page_type = match *index {
            0 => InstanceSubpageType::Quickplay,
            1 => InstanceSubpageType::Logs,
            2 => InstanceSubpageType::Mods,
            3 => InstanceSubpageType::ResourcePacks,
            4 => InstanceSubpageType::Settings,
            _ => return,
        };
        InterfaceConfig::get_mut(cx).instance_subpage = page_type;
    }))
```

### Subpage Pattern

Each tab has its own subpage component implementing `Render`:
- `InstanceLogsSubpage` → [crates/frontend/src/pages/instance/logs_subpage.rs](crates/frontend/src/pages/instance/logs_subpage.rs)
- `InstanceModsSubpage` → [crates/frontend/src/pages/instance/mods_subpage.rs](crates/frontend/src/pages/instance/mods_subpage.rs)
- `InstanceResourcePacksSubpage` → [crates/frontend/src/pages/instance/resource_packs_subpage.rs](crates/frontend/src/pages/instance/resource_packs_subpage.rs)
- `InstanceSettingsSubpage` → [crates/frontend/src/pages/instance/settings_subpage.rs](crates/frontend/src/pages/instance/settings_subpage.rs)
- `InstanceQuickplaySubpage` → [crates/frontend/src/pages/instance/quickplay_subpage.rs](crates/frontend/src/pages/instance/quickplay_subpage.rs)

---

## 2. INSTANCE DATA STRUCTURES

### Instance Playtime Data

**Frontend Layer** - [crates/bridge/src/instance.rs](crates/bridge/src/instance.rs#L47-L51)
```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InstancePlaytime {
    pub total_secs: u64,              // Total playtime in seconds
    pub current_session_secs: u64,    // Current session playtime
    pub last_played_unix_ms: Option<i64>,  // Last play timestamp (ms since UNIX epoch)
}
```

**Backend Layer** - [crates/backend/src/instance.rs](crates/backend/src/instance.rs#L55-L60)
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstanceStats {
    pub total_playtime_secs: u64,    // Total lifetime playtime
    pub session_count: u64,          // Number of times played
    pub last_played_unix_ms: Option<i64>,  // Last session timestamp
}
```

### Instance Entry Structure

**Frontend Entity** - [crates/frontend/src/entity/instance.rs](crates/frontend/src/entity/instance.rs#L212-L232)
```rust
#[derive(Clone)]
pub struct InstanceEntry {
    pub id: InstanceID,
    pub name: SharedString,
    pub icon: Option<Arc<[u8]>>,
    pub title: SharedString,
    pub root_path: Arc<Path>,
    pub dot_minecraft_folder: Arc<Path>,
    pub configuration: InstanceConfiguration,
    pub playtime: InstancePlaytime,        // ← PLAYTIME IS HERE
    pub status: InstanceStatus,            // NotRunning, Launching, Running
    pub worlds_state: BridgeDataLoadState,
    pub worlds: Entity<Arc<[InstanceWorldSummary]>>,
    pub servers_state: BridgeDataLoadState,
    pub servers: Entity<Arc<[InstanceServerSummary]>>,
    pub mods_state: BridgeDataLoadState,
    pub mods: Entity<Arc<[InstanceContentSummary]>>,
    pub resource_packs_state: BridgeDataLoadState,
    pub resource_packs: Entity<Arc<[InstanceContentSummary]>>,
}
```

### Accessing Instance Data in Components

**In a subpage component:**
```rust
let instance_entry = self.instance.read(cx);
let playtime = instance_entry.playtime;  // Access the playtime struct
let status = instance_entry.status;      // Check if running
let last_played_ms = playtime.last_played_unix_ms;
```

### Data Update Mechanism

**Message Flow:**
- Backend updates `Instance::stats` when a session ends
- Backend sends `MessageToFrontend::InstancePlaytimeUpdated` message
- Frontend updates `InstanceEntry::playtime` via `InstanceEntries::set_playtime()`
- All subscribed UI components automatically re-render

**Subscription example** - [crates/frontend/src/entity/instance.rs](crates/frontend/src/entity/instance.rs#L158-L168)
```rust
pub fn set_playtime(entity: &Entity<Self>, id: InstanceID, playtime: InstancePlaytime, cx: &mut App) {
    entity.update(cx, |entries, cx| {
        if let Some(instance) = entries.entries.get_mut(&id) {
            instance.update(cx, |instance, cx| {
                instance.playtime = playtime;
                cx.notify();  // Trigger re-render
            })
        }
    });
}
```

---

## 3. SERVER DATA STRUCTURES

### Server Data Model

**Server Summary** - [crates/bridge/src/instance.rs](crates/bridge/src/instance.rs#L55-L60)
```rust
#[derive(Debug, Clone)]
pub struct InstanceServerSummary {
    pub name: Arc<str>,
    pub ip: Arc<str>,
    pub png_icon: Option<Arc<[u8]>>,
}
```

**Key Finding:** Server summaries do **NOT currently track uptime**. Only worlds have a `last_played` timestamp.

### World Data Model (Has Time Field)

**World Summary** - [crates/bridge/src/instance.rs](crates/bridge/src/instance.rs#L48-L53)
```rust
#[derive(Debug, Clone)]
pub struct InstanceWorldSummary {
    pub title: Arc<str>,
    pub subtitle: Arc<str>,
    pub level_path: Arc<Path>,
    pub last_played: i64,              // ← TIME FIELD EXISTS
    pub png_icon: Option<Arc<[u8]>>,
}
```

### Server Uptime Implementation

**To track server uptime, you would need to:**
1. Modify `InstanceServerSummary` to add an optional `last_checked_uptime_ms` field
2. Modify backend server checking logic to timestamp when servers are verified
3. Parse server uptime info from server responses or PING packets
4. Send updates via new message variant (e.g., `InstanceServerUptimeUpdated`)

---

## 4. CURRENT TIME/DURATION TRACKING

### Where Playtime Data Flows

```
┌─────────────────────────────────────────┐
│ Backend: Instance (crates/backend/src/instance.rs)
│ - Tracks session_started_at: Option<Instant>
│ - Updates InstanceStats on session end
└────────────┬────────────────────────────┘
             │
             v
┌─────────────────────────────────────────┐
│ Bridge Message: MessageToFrontend
│ - InstancePlaytimeUpdated { id, playtime }
└────────────┬────────────────────────────┘
             │
             v
┌─────────────────────────────────────────┐
│ Frontend: InstanceEntries (entity layer)
│ - set_playtime() updates InstanceEntry
│ - Emits cx.notify() to trigger re-render
└────────────┬────────────────────────────┘
             │
             v
┌─────────────────────────────────────────┐
│ UI Component: Any subpage
│ - Reads instance.read(cx).playtime
│ - Automatically updates when data changes
└─────────────────────────────────────────┘
```

### Accessing Playtime in Components

```rust
// In a subpage's impl Render:
impl Render for InstanceStatisticsSubpage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let instance = self.instance.read(cx);
        
        let total_playtime_str = format_duration(instance.playtime.total_secs);
        let current_session_str = format_duration(instance.playtime.current_session_secs);
        let last_played = instance.playtime.last_played_unix_ms
            .map(|ms| format_timestamp(ms))
            .unwrap_or_else(|| "Never".to_string());
        
        v_flex()
            .child(div().child(format!("Total Playtime: {}", total_playtime_str)))
            .child(div().child(format!("Current Session: {}", current_session_str)))
            .child(div().child(format!("Last Played: {}", last_played)))
    }
}
```

---

## 5. IMPLEMENTATION GUIDE: ADDING A STATISTICS TAB

### Step-by-Step Implementation

#### Step 1: Create Statistics Subpage File
**File:** [crates/frontend/src/pages/instance/statistics_subpage.rs](crates/frontend/src/pages/instance/statistics_subpage.rs) (NEW)

```rust
use bridge::handle::BackendHandle;
use gpui::{prelude::*, *};
use gpui_component::{h_flex, v_flex};

use crate::{entity::instance::InstanceEntry, ts};

pub struct InstanceStatisticsSubpage {
    instance: Entity<InstanceEntry>,
}

impl InstanceStatisticsSubpage {
    pub fn new(
        instance: &Entity<InstanceEntry>,
        _backend_handle: BackendHandle,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> Self {
        Self {
            instance: instance.clone(),
        }
    }
}

impl Render for InstanceStatisticsSubpage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let instance = self.instance.read(cx);
        let theme = cx.theme();
        
        let total_secs = instance.playtime.total_secs;
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        
        let last_played_text = instance.playtime.last_played_unix_ms
            .map(|ms| {
                let duration_since = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64 - ms)
                    .unwrap_or(0);
                
                if duration_since < 60000 {
                    "Just now".to_string()
                } else if duration_since < 3600000 {
                    format!("{} minutes ago", duration_since / 60000)
                } else if duration_since < 86400000 {
                    format!("{} hours ago", duration_since / 3600000)
                } else {
                    format!("{} days ago", duration_since / 86400000)
                }
            })
            .unwrap_or_else(|| ts!("instance.statistics.never").to_string());
        
        v_flex()
            .size_full()
            .p_4()
            .gap_4()
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::Bold)
                    .child(ts!("instance.statistics"))
            )
            .child(
                h_flex()
                    .gap_4()
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().text_color(theme.text_muted).child(ts!("instance.statistics.playtime")))
                            .child(div().text_base().child(format!("{}h {}m {}s", hours, minutes, seconds)))
                    )
            )
            .child(
                h_flex()
                    .gap_4()
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().text_color(theme.text_muted).child(ts!("instance.statistics.last_played")))
                            .child(div().text_base().child(last_played_text))
                    )
            )
    }
}
```

#### Step 2: Update InstanceSubpageType Enum
**File:** [crates/frontend/src/pages/instance/instance_page.rs](crates/frontend/src/pages/instance/instance_page.rs#L176-L190)

Add `Statistics` variant:
```rust
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceSubpageType {
    #[default]
    Quickplay,
    Logs,
    Mods,
    ResourcePacks,
    Settings,
    Statistics,  // ← ADD THIS
}
```

#### Step 3: Update create() Method
In `InstanceSubpageType::create()`:
```rust
InstanceSubpageType::Statistics => InstanceSubpage::Statistics(cx.new(|cx| {
    InstanceStatisticsSubpage::new(instance, backend_handle, window, cx)
})),
```

#### Step 4: Update InstanceSubpage Enum
```rust
#[derive(Clone)]
pub enum InstanceSubpage {
    Quickplay(Entity<InstanceQuickplaySubpage>),
    Logs(Entity<InstanceLogsSubpage>),
    Mods(Entity<InstanceModsSubpage>),
    ResourcePacks(Entity<InstanceResourcePacksSubpage>),
    Settings(Entity<InstanceSettingsSubpage>),
    Statistics(Entity<InstanceStatisticsSubpage>),  // ← ADD THIS
}
```

#### Step 5: Update page_type() Method
```rust
pub fn page_type(&self) -> InstanceSubpageType {
    match self {
        InstanceSubpage::Quickplay(_) => InstanceSubpageType::Quickplay,
        InstanceSubpage::Logs(_) => InstanceSubpageType::Logs,
        InstanceSubpage::Mods(_) => InstanceSubpageType::Mods,
        InstanceSubpage::ResourcePacks(_) => InstanceSubpageType::ResourcePacks,
        InstanceSubpage::Settings(_) => InstanceSubpageType::Settings,
        InstanceSubpage::Statistics(_) => InstanceSubpageType::Statistics,  // ← ADD THIS
    }
}
```

#### Step 6: Update into_any_element() Method
```rust
pub fn into_any_element(self) -> AnyElement {
    match self {
        Self::Quickplay(entity) => entity.into_any_element(),
        Self::Logs(entity) => entity.into_any_element(),
        Self::Mods(entity) => entity.into_any_element(),
        Self::ResourcePacks(entity) => entity.into_any_element(),
        Self::Settings(entity) => entity.into_any_element(),
        Self::Statistics(entity) => entity.into_any_element(),  // ← ADD THIS
    }
}
```

#### Step 7: Update Render Method in InstancePage
In `impl Render for InstancePage`:
```rust
let selected_index = match &self.subpage {
    InstanceSubpage::Quickplay(_) => 0,
    InstanceSubpage::Logs(_) => 1,
    InstanceSubpage::Mods(_) => 2,
    InstanceSubpage::ResourcePacks(_) => 3,
    InstanceSubpage::Settings(_) => 4,
    InstanceSubpage::Statistics(_) => 5,  // ← ADD THIS
};

// And in the TabBar:
.child(Tab::new().label(ts!("instance.statistics")))  // ← ADD THIS

// And in the on_click handler:
.on_click(cx.listener(|_, index, _, cx| {
    let page_type = match *index {
        0 => InstanceSubpageType::Quickplay,
        1 => InstanceSubpageType::Logs,
        2 => InstanceSubpageType::Mods,
        3 => InstanceSubpageType::ResourcePacks,
        4 => InstanceSubpageType::Settings,
        5 => InstanceSubpageType::Statistics,  // ← ADD THIS
        _ => return,
    };
    InterfaceConfig::get_mut(cx).instance_subpage = page_type;
}))
```

#### Step 8: Add Module Export
**File:** [crates/frontend/src/pages/instance/mod.rs](crates/frontend/src/pages/instance/mod.rs)

```rust
pub mod statistics_subpage;  // ← ADD THIS
```

#### Step 9: Update Imports in instance_page.rs
```rust
use crate::{
    entity::{DataEntities, instance::InstanceEntry},
    // ... other imports ...
    pages::instance::{
        logs_subpage::InstanceLogsSubpage,
        mods_subpage::InstanceModsSubpage,
        quickplay_subpage::InstanceQuickplaySubpage,
        resource_packs_subpage::InstanceResourcePacksSubpage,
        settings_subpage::InstanceSettingsSubpage,
        statistics_subpage::InstanceStatisticsSubpage,  // ← ADD THIS
    },
    // ... rest of imports ...
};
```

---

## 6. REQUIRED TRANSLATIONS

Add these keys to your translation files:

```
instance.statistics = "Statistics"
instance.statistics.playtime = "Total Playtime"
instance.statistics.sessions = "Sessions Played"
instance.statistics.last_played = "Last Played"
instance.statistics.never = "Never"
instance.statistics.just_now = "Just now"
instance.statistics.minutes_ago = "{} minutes ago"
instance.statistics.hours_ago = "{} hours ago"
instance.statistics.days_ago = "{} days ago"
```

---

## 7. OPTIONAL ENHANCEMENTS

### Server Uptime Tracking
To add server uptime:
1. Modify [crates/bridge/src/instance.rs](crates/bridge/src/instance.rs) `InstanceServerSummary` to add time fields
2. Add server connectivity checking logic in backend
3. Implement PING-based uptime detection

### World Statistics
Access world statistics from quickplay data:
```rust
let worlds = self.instance.read(cx).worlds.read(cx);
for world in worlds.iter() {
    println!("World: {}, Last Played: {}", world.title, world.last_played);
}
```

### Session History
Backend `InstanceStats` already tracks:
- `session_count: u64` - number of times played
- `total_playtime_secs: u64` - lifetime playtime
- `last_played_unix_ms: Option<i64>` - most recent session

These could be displayed in the statistics tab.

---

## 8. FILE STRUCTURE REFERENCE

```
crates/
├── backend/src/
│   └── instance.rs           ← InstanceStats, Instance::stats field
├── bridge/src/
│   └── instance.rs           ← InstancePlaytime, MessageToFrontend
├── frontend/src/
│   ├── entity/
│   │   └── instance.rs       ← InstanceEntry, set_playtime()
│   ├── interface_config.rs   ← InterfaceConfig, instance_subpage field
│   └── pages/instance/
│       ├── instance_page.rs  ← Tab management, InstancePage
│       ├── mod.rs            ← Module exports
│       ├── logs_subpage.rs
│       ├── mods_subpage.rs
│       ├── quickplay_subpage.rs
│       ├── resource_packs_subpage.rs
│       ├── settings_subpage.rs
│       └── statistics_subpage.rs  ← NEW FILE TO CREATE
└── schema/src/
    └── instance.rs           ← InstanceConfiguration, InstanceStats serialization
```

---

## 9. KEY ARCHITECTURAL INSIGHTS

1. **Reactive UI:** GPUI uses reactive entities - updating data automatically triggers re-renders
2. **Persistent State:** InterfaceConfig automatically saves to disk
3. **Message-Driven:** Frontend and backend communicate via bridge messages
4. **Type Safety:** Rust's type system ensures compile-time verification of data flow
5. **Entity Pattern:** Each component is an `Entity<T>` that can be read/updated reactively

---

## 10. TESTING THE IMPLEMENTATION

1. Create the `statistics_subpage.rs` file
2. Update all enum and match statements
3. Add module export
4. Compile: `cargo build --release`
5. The new tab should appear as "Statistics" in the instance view
6. Tab selection should persist across app restarts

