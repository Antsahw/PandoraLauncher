# Quick Reference: Cross-Window Communication in PandoraLauncher

## Can I Access GameOutputRoot from ServerLogsSubpage?

**Answer: No** ❌

### Why?
```
Window 1: Main Window          Window 2: Game Output Window
┌──────────────────────┐      ┌──────────────────────┐
│ ServerPage           │      │ GameOutputRoot       │
│  └─ ServerSubpage    │      │  └─ GameOutput       │
│      └─ ServerLogs   │      │      └─ logs, scroll │
└──────────────────────┘      └──────────────────────┘
        ❌ No direct path ❌
```

Different `Context` windows = different entity namespaces = no `.update()` across them.

### Proof
```rust
// This is what Processor has:
game_output_root: Option<Entity<GameOutputRoot>>,      // ← Can access this
game_output_window: Option<WindowHandle<Root>>,         // ← Using this

// This is what ServerLogsSubpage has:
backend_handle: BackendHandle,                          // Can only message backend
_data: DataEntities,                                    // No GameOutputRoot here

// Processor can do this:
root_entity.update(cx, |root, cx| { ... })  // Cross-window works here
// Because Processor has both the entity AND the same App context

// ServerLogsSubpage CANNOT do this:
game_output_root.update(cx, ...)            // Where does it get game_output_root?
                                            // Not in DataEntities or ServerPage
```

---

## Existing Communication Paths

### ✅ Path 1: Processor ← → GameOutputRoot
**How**: Processor stores Entity references + WindowHandle
```rust
// processor.rs:12-17
game_output_window: Option<WindowHandle<Root>>,
game_output_root: Option<Entity<GameOutputRoot>>,
game_output_tabs: HashMap<usize, Entity<GameOutput>>,
```
**Usage**: `root_entity.update(cx, |root, cx| { ... })`
**Why**: Both accessed from Processor's context with App mutability

### ✅ Path 2: ServerPage → Backend
**How**: BackendHandle cloned to all UI components
```rust
backend_handle: BackendHandle,  // In DataEntities → in ServerPage
```
**Usage**: `backend_handle.send(MessageToBackend::...)`
**Why**: BackendHandle is Send+Sync, works across threads

### ✅ Path 3: Backend → Any UI Component
**How**: MessageToFrontend channel routed through Processor
```rust
Processor::process() match { MessageToFrontend::AddGameOutput => ... }
```
**Usage**: Backend sends, Processor routes
**Why**: One-way channel, all UI gets updates same way

### ❌ Path 4: ServerPage ← → GameOutputRoot
**How**: No direct mechanism exists
**Why**: No shared handle, different contexts

---

## Shared State Patterns in Codebase

| Pattern | Example | Thread-Safe | Use Case |
|---------|---------|-------------|----------|
| `Rc<RefCell<T>>` | scroll_state | Single-threaded | Local mutable state |
| `Arc<T>` | Arc<[Arc<str>]> | Read-only | Immutable shared data |
| `Arc<RwLock<T>>` | panic_message | Thread-safe | Shared mutable state |
| `Entity<T>` | GameOutput | GPUI-manages | All UI components |
| `Global for T` | InterfaceConfig | App-wide | Singletons |
| `HashMap` | Processor.tabs | Owned by one | Routing/lookup |

**KEY**: EntityContext is separate per-window. Can't lock an Entity across windows.

---

## Solution: Backend Subscription Model

### Instead of:
```rust
❌ "Get the GameOutputRoot from window 2 and read its logs"
   - Impossible: different contexts
   - Wrong: tight coupling
   - Fragile: breaks if window closes
```

### Do:
```rust
✅ "Backend sends logs to BOTH GameOutput and ServerLogsSubpage"
   1. ServerLogsSubpage: subscribe to server logs → SubscribeToServerLogs
   2. Backend: register subscriber
   3. Log reader: send to all subscribers → AddServerLog (new message type)
   4. Processor: route to ServerLogsSubpage
   5. ServerLogsSubpage: update display
```

### Architecture
```
Backend (source)
    ↓
Log Reader Thread
    ├→ AddGameOutput → Processor → GameOutputRoot (existing)
    └→ AddServerLog → Processor → ServerLogsSubpage (new)
```

### Benefits
- ✅ Works even if GameOutputRoot window closed
- ✅ Both receive in real-time
- ✅ Can add more subscribers later
- ✅ Backend is single source of truth
- ✅ No cross-window coupling
- ✅ GPUI-idiomatic

---

## Implementation Checklist

- [ ] **Message Protocol** (15 min)
  - [ ] Add `SubscribeToServerLogs` to MessageToBackend
  - [ ] Add `UnsubscribeFromServerLogs` to MessageToBackend
  - [ ] Add `AddServerLog` to MessageToFrontend

- [ ] **Backend Subscription** (1-2 hours)
  - [ ] Track subscribers per server
  - [ ] Handle subscribe/unsubscribe messages
  - [ ] Send AddServerLog to all subscribers

- [ ] **Frontend Routing** (30 min)
  - [ ] Add case in Processor::process()
  - [ ] Route AddServerLog to ServerLogsSubpage

- [ ] **UI Display** (30 min)
  - [ ] Add subscribe task to ServerLogsSubpage
  - [ ] Implement on_server_log() method
  - [ ] Append logs to display

- [ ] **Cleanup** (15 min)
  - [ ] Unsubscribe on drop
  - [ ] Remove from Processor tracking

---

## Key Files To Modify

```
crates/bridge/src/message.rs
  ├─ MessageToBackend enum ... add variants
  └─ MessageToFrontend enum ... add variant

crates/backend/src/backend_handler.rs
  ├─ BackendHandler struct ... add subscribers HashMap
  ├─ handle_message() ... add subscription cases
  └─ start_server() ... send to subscribers

crates/frontend/src/processor.rs
  ├─ Processor struct ... add server_logs_pages lookup
  └─ process() method ... add AddServerLog case

crates/frontend/src/pages/instance/server_page.rs
  ├─ ServerLogsSubpage struct ... add _log_subscription task
  ├─ new() method ... subscribe to logs
  ├─ on_server_log() method ... append to display
  └─ drop() ... unsubscribe
```

---

## Why NOT These Alternatives?

| Option | Problem |
|--------|---------|
| Direct Entity Access | Can't cross windows, different contexts |
| Global Arc<GameOutputRoot> | Breaks entity lifecycle, not GPUI-compliant |
| Global Arc<RwLock<Logs>> | Synchronization overhead, cumbersome lookup |
| Query-on-Demand | Not real-time, races |
| Cache in Processor | No way to pass to ServerLogsSubpage |

---

## Code Location Reference

| What | Where |
|-----|-------|
| How Processor stores GameOutputRoot | processor.rs:280 |
| How Processor routes messages | processor.rs:312-335 |
| GameOutput::collect_logs() | game_output/mod.rs:97 |
| ServerLogsSubpage current render | server_page.rs:300 |
| AddGameOutput message | bridge/src/message.rs:315 |
| Processor::process() main router | processor.rs:65 |
| Log reader backend | backend/src/log_reader.rs:43 |

---

## Testing Plan

1. **Basic functionality**
   - Start server from ServerPage
   - Verify logs appear with timestamp + level
   - Commands still work

2. **Multi-window**
   - Open GameOutput window
   - Verify logs in BOTH places
   - No duplication

3. **Edge cases**
   - Close ServerPage → verify unsubscribe
   - Close GameOutput → logs still in ServerPage
   - Multiple servers → no cross-contamination

4. **Performance**
   - Large volume logs → no UI freeze
   - Subscribe/unsubscribe → no leaks

---

## Estimated Work

| Task | Time | Difficulty |
|------|------|-----------|
| Messages | 15 min | ⭐ Easy |
| Backend | 1-2 hr | ⭐⭐ Medium |
| Frontend Router | 30 min | ⭐ Easy |
| UI Display | 30 min | ⭐⭐ Medium |
| Testing | 30 min | ⭐ Easy |
| **TOTAL** | **3-4 hr** | ⭐⭐ Medium |

---

## Compare to Other Launchers

**How do other launchers handle this?**

| Launcher | Approach |
|----------|----------|
| Minecraft Launcher | Single window, all logs in one place |
| MultiMC | Server logs separate window, independent receiver |
| Prism Launcher | Server logs tab in main window, messages queued |

**Our approach (subscription)** = similar to Prism (queued) + MultiMC (separate window)
