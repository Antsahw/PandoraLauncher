# Technic Integration Search Implementation Analysis

## Overview

The Technic Platform modpack search functionality has been partially implemented across the codebase. The frontend UI and API schema are complete, but some backend implementation details need attention.

---

## File Locations & Architecture

### Schema Definitions
**File:** [crates/schema/src/technic.rs](crates/schema/src/technic.rs)

**API Endpoints:**
- `TECHNIC_SEARCH_URL`: `https://api.technicpack.net/search`
- `TECHNIC_MODPACK_URL`: `https://api.technicpack.net/modpack`

**Request/Response Types:**

```rust
// Search Request
pub struct TechnicSearchRequest {
    pub query: Option<Arc<str>>,      // Search query
    pub offset: usize,                 // Pagination offset
    pub limit: usize,                  // Results limit (typically 50)
}

// Search Result Hit
pub struct TechnicSearchHit {
    pub name: Arc<str>,                // Modpack slug name
    pub display_name: Option<Arc<str>>, // Human-readable name
    pub description: Option<Arc<str>>,
    pub author: Option<Arc<str>>,
    pub icon: Option<Arc<str>>,        // Icon URL
    pub logo: Option<Arc<str>>,        // Logo URL
    pub url: Option<Arc<str>>,         // Modpack URL
    pub recommended: Option<Arc<str>>, // Recommended version
    pub latest: Option<Arc<str>>,      // Latest version
    pub downloads: Option<u64>,
    pub featured: bool,
}

// Search Result Wrapper
pub struct TechnicSearchResult {
    pub results: Arc<[TechnicSearchHit]>,
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
}

// Modpack Info Request
pub struct TechnicModpackRequest {
    pub name: Arc<str>,  // Modpack name to fetch
}

// Modpack Info Detail
pub struct TechnicModpackInfo {
    pub name: Arc<str>,
    pub display_name: Option<Arc<str>>,
    pub description: Option<Arc<str>>,
    pub author: Option<Arc<str>>,
    pub icon: Option<Arc<str>>,
    pub logo: Option<Arc<str>>,
    pub url: Option<Arc<str>>,
    pub recommended: Option<Arc<str>>,
    pub latest: Option<Arc<str>>,
    pub downloads: Option<u64>,
    pub versions: Option<Arc<[TechnicVersion]>>,  // Available versions
}

// Version Information
pub struct TechnicVersion {
    pub version: Arc<str>,
    pub minecraft_version: Option<Arc<str>>,
    pub java_version: Option<Arc<str>>,
    pub build: Option<u32>,
    pub released: Option<Arc<str>>,
}

// Modpack Manifest from Technic files
pub struct TechnicPackManifestJson {
    pub name: Option<Arc<str>>,
    pub version: Arc<str>,
    pub minecraft: Option<Arc<str>>,
    pub java: Option<Arc<str>>,
    pub mods: Arc<[Arc<str>]>,
    pub java_args: Arc<[Arc<str>]>,
}
```

---

## Frontend Implementation

### Search Page UI
**File:** [crates/frontend/src/pages/technic_page.rs](crates/frontend/src/pages/technic_page.rs)

**Components:**
```rust
pub struct TechnicSearchPage {
    data: DataEntities,
    hits: Vec<schema::technic::TechnicSearchHit>,  // Search results
    install_for: Option<InstanceID>,               // Target instance
    search_state: Entity<InputState>,              // Input field state
    loading: Option<Entity<FrontendMetadataState>>, // Loading state
    search_error: Option<SharedString>,            // Error message
    pending_reload: bool,
    last_search: Arc<str>,
}
```

**Search Flow:**

1. **Input Handling** - `on_search_input_event()`:
   - Monitors text input changes
   - Trims whitespace and deduplicates identical searches
   - Sets `pending_reload` flag when input changes
   - Clears previous errors

2. **Search Loading** - `load_search()`:
   - Creates `TechnicSearchRequest` with:
     - Query from input (or `None` if empty)
     - `offset: 0`
     - `limit: 50`
   - Requests metadata via `FrontendMetadata::request()`
   - Handles three states:
     - **Loading**: Subscribes to metadata updates
     - **Loaded**: Updates `hits` vector with results
     - **Error**: Stores error message in `search_error`
   - Handles pending reloads after subscription

3. **Rendering**:
   - Shows error alert if search fails
   - Shows placeholder text if no results and not loading
   - Renders results as divs with modpack information

**Key Features:**
- Live search with input debouncing
- Error handling and display
- Instance selection for installation
- Installed status tracking (partially implemented)

---

## Bridge Layer

### Metadata Request/Response Types
**File:** [crates/bridge/src/meta.rs](crates/bridge/src/meta.rs)

**Request Types:**
```rust
pub enum MetadataRequest {
    // ... other variants
    TechnicSearch(TechnicSearchRequest),
    TechnicModpackInfo(TechnicModpackRequest),
}
```

**Response Types:**
```rust
pub enum MetadataResult {
    // ... other variants
    TechnicSearchResult(Arc<TechnicSearchResult>),
    TechnicModpackInfoResult(Arc<TechnicModpackInfo>),
}
```

---

## Backend API Implementation

### Search API Calls
**File:** [crates/backend/src/metadata/items.rs](crates/backend/src/metadata/items.rs)

**TechnicSearchMetadataItem Implementation:**

```rust
pub struct TechnicSearchMetadataItem<'a>(pub &'a TechnicSearchRequest);

impl<'a> MetadataItem for TechnicSearchMetadataItem<'a> {
    type T = TechnicSearchResult;

    fn request(&self, client: &reqwest::Client) -> RequestBuilder {
        let mut req = client.get(TECHNIC_SEARCH_URL);
        if let Some(query) = &self.0.query {
            req = req.query(&[("q", query.as_ref())]);  // Query parameter
        }
        req = req.query(&[("offset", &self.0.offset.to_string())])
            .query(&[("limit", &self.0.limit.to_string())]);
        req
    }

    fn expires(&self) -> bool {
        true  // Cache results with expiration
    }

    fn state(&self, states: &mut MetadataManagerStates) -> MetaLoadStateWrapper<Self::T> {
        states.technic_search.entry(self.0.clone()).or_default().clone()
    }

    fn deserialize(bytes: &[u8]) -> Result<Self::T, MetaLoadError> {
        Ok(serde_json::from_slice(bytes)?)
    }
}
```

**TechnicModpackInfoMetadataItem Implementation:**

```rust
pub struct TechnicModpackInfoMetadataItem<'a>(pub &'a TechnicModpackRequest);

impl<'a> MetadataItem for TechnicModpackInfoMetadataItem<'a> {
    type T = TechnicModpackInfo;

    fn request(&self, client: &reqwest::Client) -> RequestBuilder {
        // URL: https://api.technicpack.net/modpack/{modpack-name}
        client.get(format!("{}/{}", TECHNIC_MODPACK_URL, self.0.name))
    }

    fn expires(&self) -> bool {
        true
    }

    fn state(&self, states: &mut MetadataManagerStates) -> MetaLoadStateWrapper<Self::T> {
        states.technic_modpack_info.entry(self.0.clone()).or_default().clone()
    }

    fn deserialize(bytes: &[u8]) -> Result<Self::T, MetaLoadError> {
        Ok(serde_json::from_slice(bytes)?)
    }
}
```

**Caching Manager:**
**File:** [crates/backend/src/metadata/manager.rs](crates/backend/src/metadata/manager.rs)

```rust
pub struct MetadataManagerStates {
    // ... other fields
    pub(super) technic_search: HashMap<schema::technic::TechnicSearchRequest, MetaLoadStateWrapper<schema::technic::TechnicSearchResult>>,
    pub(super) technic_modpack_info: HashMap<schema::technic::TechnicModpackRequest, MetaLoadStateWrapper<schema::technic::TechnicModpackInfo>>,
}
```

**Backend Handler Integration:**
**File:** [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs#L67-L73)

```rust
bridge::meta::MetadataRequest::TechnicSearch(ref search) => {
    let (result, handle) = meta.fetch_with_keepalive(
        &crate::metadata::items::TechnicSearchMetadataItem(search), 
        force_reload
    ).await;
    (result.map(MetadataResult::TechnicSearchResult), handle)
},
bridge::meta::MetadataRequest::TechnicModpackInfo(ref modpack) => {
    let (result, handle) = meta.fetch_with_keepalive(
        &crate::metadata::items::TechnicModpackInfoMetadataItem(modpack), 
        force_reload
    ).await;
    (result.map(MetadataResult::TechnicModpackInfoResult), handle)
},
```

---

## Current Search Implementation Summary

### API Endpoint Details

**Search Query:**
- **URL:** `https://api.technicpack.net/search`
- **Method:** GET
- **Parameters:**
  - `q`: Search query string (optional, omitted if empty)
  - `offset`: Result offset for pagination (default: 0)
  - `limit`: Number of results to return (typically 50)

**Example:**
```
GET https://api.technicpack.net/search?q=modpack_name&offset=0&limit=50
```

**Modpack Info Query:**
- **URL:** `https://api.technicpack.net/modpack/{modpack-name}`
- **Method:** GET
- **Example:**
```
GET https://api.technicpack.net/modpack/technic
```

### Error Handling

**Location:** [crates/frontend/src/entity/metadata.rs](crates/frontend/src/entity/metadata.rs#L135-L170)

```rust
impl AsMetadataResult<Arc<schema::technic::TechnicSearchResult>> for FrontendMetadataState {
    fn result(&self) -> FrontendMetadataResult<'_, Arc<schema::technic::TechnicSearchResult>> {
        match self {
            FrontendMetadataState::Loading => FrontendMetadataResult::Loading,
            FrontendMetadataState::Loaded { result, .. } => {
                match result {
                    Ok(MetadataResult::TechnicSearchResult(result)) => FrontendMetadataResult::Loaded(result),
                    Ok(_) => FrontendMetadataResult::Error(ts!("system.metadata_error")),
                    Err(error) => FrontendMetadataResult::Error(SharedString::new(error.clone())),
                }
            },
        }
    }
}

impl AsMetadataResult<Arc<schema::technic::TechnicModpackInfo>> for FrontendMetadataState {
    fn result(&self) -> FrontendMetadataResult<'_, Arc<schema::technic::TechnicModpackInfo>> {
        match self {
            FrontendMetadataState::Loading => FrontendMetadataResult::Loading,
            FrontendMetadataState::Loaded { result, .. } => {
                match result {
                    Ok(MetadataResult::TechnicModpackInfoResult(result)) => FrontendMetadataResult::Loaded(result),
                    Ok(_) => FrontendMetadataResult::Error(ts!("system.metadata_error")),
                    Err(error) => FrontendMetadataResult::Error(SharedString::new(error.clone())),
                }
            },
        }
    }
}
```

### Content Source Tracking

**Location:** [crates/schema/src/content.rs](crates/schema/src/content.rs)

```rust
pub enum ContentSource {
    Manual,
    ModrinthUnknown,
    ModrinthProject { project_id: Arc<str> },
    CurseforgeProject { project_id: u32 },
    TechnicModpack { modpack_name: Arc<str> },  // Tracks Technic source
}
```

### Modpack Serialization

**Location:** [crates/backend/src/mod_metadata.rs](crates/backend/src/mod_metadata.rs#L955+)

```rust
ContentSource::TechnicModpack { modpack_name } => {
    data.push(4_u8);  // Type marker
    if modpack_name.len() > 127 {
        panic!("technic modpack name was unexpectedly big: {:?}", &modpack_name);
    }
    data.push(modpack_name.len() as u8);
    data.extend_from_slice(modpack_name.as_bytes());
}
```

---

## Known Issues & TODO Items

### Backend Completion Tasks

1. **Modpack Installation** (`crates/backend/src/install_content.rs`)
   - Location: [Line 715+](crates/backend/src/install_content.rs#L715)
   - Status: TODO - Full Technic modpack download and extraction not implemented
   - Currently returns error: "not yet supported"

2. **Update Checking** (`crates/backend/src/backend_handler.rs`)
   - Location: [Line 1278](crates/backend/src/backend_handler.rs#L1278)
   - Status: TODO - Technic modpack update checking not implemented
   - Needs version comparison logic similar to CurseForge/Modrinth

3. **Modpack Metadata Loading** (`crates/backend/src/mod_metadata.rs`)
   - Status: TODO - Needs manifest.json parsing from Technic archives
   - Should extract and validate modpack information

### Frontend Completion Tasks

1. **Install Flow UI** - Implementation needed
2. **Version Selection** - UI for choosing modpack versions
3. **Installed Status Indicator** - Show "Installed" badge in search results
4. **Update Available Indicator** - Show when newer versions exist

---

## Integration Points

### Page Navigation
**File:** [crates/frontend/src/ui.rs](crates/frontend/src/ui.rs)

```rust
PageType::Technic { installing_for } => {
    let installing_for = installing_for.as_ref().and_then(|name| 
        InstanceEntries::find_id_by_name(&data.instances, name, cx)
    );

    let page = cx.new(|cx| {
        TechnicSearchPage::new(installing_for, data, window, cx)
    });
    Ok(LauncherPage::Technic(page))
}
```

### Module Exports
**File:** [crates/frontend/src/pages/mod.rs](crates/frontend/src/pages/mod.rs)

```rust
pub mod technic_page;
```

---

## Summary

**✅ Completed:**
- Schema types for search and modpack info
- Frontend search UI with live input
- API endpoint definitions
- Backend metadata request/response handling
- Error handling framework
- Caching infrastructure

**⚠️ Partially Implemented:**
- Search display (basic hit rendering)
- Installed status tracking skeleton

**❌ Not Yet Implemented:**
- Modpack installation flow
- Download handling and extraction
- Update checking system
- Version selection UI
- Manifest parsing from archives
