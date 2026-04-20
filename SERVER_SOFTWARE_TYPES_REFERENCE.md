# Server Software Types - Codebase Location Reference

## Summary
Server software types (Paper, Purpur, Fabric, Forge, NeoForge) are defined in multiple locations throughout the codebase, primarily in the frontend UI and backend implementation.

---

## Primary Definition Locations

### 1. **Frontend: ServerLoader Enum** (UI Selection)
**File:** [crates/frontend/src/modals/create_server.rs](crates/frontend/src/modals/create_server.rs#L13-L18)
**Lines:** 13-18

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

**Purpose:** Enum used in the frontend modal (`CreateServerModalState`) to track the selected server software type during server creation.

**Related Code:**
- **Initialization:** [Line 84](crates/frontend/src/modals/create_server.rs#L84) - Defaults to `Paper`
- **UI Rendering:** [Lines 253-283](crates/frontend/src/modals/create_server.rs#L253-L283) - Button group with 5 options
- **Button Selection Logic:** [Lines 279-283](crates/frontend/src/modals/create_server.rs#L279-L283) - Maps button clicks to enum values
- **Conversion to String:** [Lines 338-350](crates/frontend/src/modals/create_server.rs#L338-L350) - Converts enum to string for backend:
  - `ServerLoader::Paper` → `"Paper"`
  - `ServerLoader::Purpur` → `"Purpur"`
  - `ServerLoader::Fabric` → `"Fabric"`
  - `ServerLoader::Forge` → `"Forge"`
  - `ServerLoader::NeoForge` → `"NeoForge"`

---

### 2. **Backend: String-based Server Software** (Core Implementation)
**File:** [crates/backend/src/backend.rs](crates/backend/src/backend.rs#L1152)
**Key Functions:**

#### validate_server_version()
**Lines:** 1093-1140
```rust
pub async fn validate_server_version(&self, version: &str, server_software: &str) -> bool {
    match server_software {
        "Paper" => { ... },
        "Purpur" => { ... },
        "Fabric" => { ... },
        "Forge" | "NeoForge" => { true }, // Accept any version
        _ => false,
    }
}
```

This function validates if a given Minecraft version is available for the specified server software by querying respective APIs.

#### create_server()
**Lines:** 1152-1240
```rust
pub async fn create_server(
    &self, 
    name: &str, 
    version: &str, 
    server_software: &str,  // String representation: "Paper", "Purpur", etc.
    icon: Option<EmbeddedOrRaw>, 
    modal_action: &bridge::modal_action::ModalAction
) -> Option<PathBuf>
```

**Key Logic:**
- Validates server software (lines 1161-1166)
- Creates server directory structure
- Creates `server_metadata.json`, `eula.txt`, `server.properties`
- Creates `plugins/` directory only for Paper/Purpur (lines 1215-1217)
- Creates `README.md` with download instructions (lines 1219-1227)

#### download_server_jar_background()
**Lines:** 1586-1768
Handles async JAR download with match statement:
```rust
match software {
    "Paper" => { /* Download from https://api.papermc.io */ },
    "Purpur" => { /* Download from https://api.purpurmc.io */ },
    "Fabric" => { /* Download from https://meta.fabricmc.net */ },
    "Forge" => { /* Download from Maven repo */ },
    "NeoForge" => { /* Download from NeoForge Maven */ },
    _ => { /* Error */ }
}
```

---

### 3. **Message Bridge: CreateServer Message**
**File:** [crates/bridge/src/message.rs](crates/bridge/src/message.rs#L39-L43)

The frontend sends a `MessageToBackend::CreateServer`:
```rust
MessageToBackend::CreateServer {
    name: SharedString,
    version: SharedString,
    server_software: SharedString,  // "Paper", "Purpur", etc.
    icon: Option<EmbeddedOrRaw>,
    modal_action: ModalAction,
}
```

---

### 4. **Instance Configuration Storage**
**File:** [crates/schema/src/instance.rs](crates/schema/src/instance.rs)

Instance configuration stores server software as string in metadata:
```json
{
    "name": "server_name",
    "server_software": "Paper|Purpur|Fabric|Forge|NeoForge",
    "minecraft_version": "1.20.1"
}
```

---

## Related Type Definitions

### Loader Enum (for Client/Mod Loaders - Different from Server Software)
**File:** [crates/schema/src/loader.rs](crates/schema/src/loader.rs#L6-L18)

This defines mod loaders (for Minecraft clients), NOT server software:
```rust
pub enum Loader {
    Vanilla,
    Fabric,
    Forge,
    NeoForge,
    Unknown,
}
```

---

## Frontend UI Components

### Server Settings Page
**File:** [crates/frontend/src/pages/instance/server_page.rs](crates/frontend/src/pages/instance/server_page.rs)

Renders server pages with tabs:
- Logs
- Settings  
- Properties

### Server Configuration Subpage
**File:** [crates/frontend/src/pages/instance/server_settings_subpage.rs](crates/frontend/src/pages/instance/server_settings_subpage.rs)

Handles runtime server configuration and settings.

---

## API Endpoints by Server Software

### Paper
- **Endpoint:** `https://api.papermc.io/v2/projects/paper/versions/{version}`
- **JAR Download:** `https://api.papermc.io/v2/projects/paper/versions/{version}/builds/{build}/downloads/paper-{version}-{build}.jar`

### Purpur
- **Endpoint:** `https://api.purpurmc.io/v2/purpur/{version}`
- **JAR Download:** `https://api.purpurmc.io/v2/purpur/{version}/builds/{build}/downloads/purpur-{version}-{build}.jar`
- **Fallback:** Falls back to Paper if Purpur build not available

### Fabric
- **Endpoint:** `https://meta.fabricmc.net/v2/versions/loader/{version}`
- **JAR Download:** `https://meta.fabricmc.net/v2/versions/loader/{version}/{loader}/server/jar`

### Forge
- **Maven Repository:** Uses Maven central and Forge Maven
- **Complex installation:** Requires installer JAR execution

### NeoForge
- **Maven Repository:** `https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml`
- **Complex installation:** Requires installer JAR execution

---

## File Locations Summary

| Component | File | Lines | Purpose |
|-----------|------|-------|---------|
| **Frontend Enum** | `crates/frontend/src/modals/create_server.rs` | 13-18 | UI dropdown definition |
| **Backend String Match** | `crates/backend/src/backend.rs` | 1093-1768 | Version validation, server creation, JAR download |
| **Message Definition** | `crates/bridge/src/message.rs` | 39-43 | IPC between frontend/backend |
| **Server Config Schema** | `crates/schema/src/server_config.rs` | - | Java runtime settings only (not software type) |
| **Instance Config** | `crates/schema/src/instance.rs` | - | Persists server metadata including software type |
| **Server Page UI** | `crates/frontend/src/pages/instance/server_page.rs` | - | Server display and management |

---

## Key Observations

1. **Dual Representation:**
   - **Frontend:** Uses `ServerLoader` enum for type safety in UI
   - **Backend:** Uses string `"Paper"`, `"Purpur"`, etc. for flexibility

2. **String-based Backend:**
   - Backend uses match statements on string slices: `match server_software { ... }`
   - Allows easier addition of new server types without recompilation

3. **No Centralized Enum in Backend:**
   - Backend doesn't define an enum for server types
   - Uses strings and pattern matching instead for backend/frontend decoupling

4. **Hardcoded in Multiple Locations:**
   - Server types are hardcoded in create_server.rs UI
   - Hardcoded in backend.rs download logic
   - Hardcoded in message.rs bridge

5. **Extensibility:**
   - To add a new server type, need to update:
     1. `ServerLoader` enum in create_server.rs
     2. UI button group rendering logic
     3. Backend string matching in validate_server_version()
     4. Backend string matching in download_server_jar_background()
     5. Download implementation logic
