# Technic Platform Integration - Implementation Guide

## Summary of Work Completed

I've successfully implemented the frontend and schema infrastructure for Technic Platform modpack integration in your PandoraLauncher. Here's what has been done:

### ✅ Completed:

1. **Schema Types** (`crates/schema/src/technic.rs`)
   - Search request/response structures matching Technic API
   - Modpack info types with version support  
   - Manifest JSON structures for modpack parsing
   - Cached file info for integrity checking

2. **Bridge Layer Updates**
   - `ContentDownload::Technic` variant for download specification
   - `ContentSource::TechnicModpack` for tracking modpack source
   - Metadata requests added for search and modpack info
   - Corresponding metadata result types

3. **Frontend Pages & Components**
   - Technic search page with live search UI
   - Modpack install modal with instance selection
   - Results display with installed status tracking
   - "Install from Technic" button on mods subpage

4. **UI Integration**
   - PageType enum updated with Technic variant
   - LauncherPage enum updated
   - Full page navigation integrated
   - Module exports configured

## Remaining Implementation Work

### 1. **Backend Metadata System** (Most Important)

Location: `crates/backend/src/meta.rs` or similar metadata handler

You need to implement API calls to Technic Platform:

```rust
// Example API endpoints needed:
// GET https://api.technicpack.net/search?q={query}&offset=0&limit=50
// GET https://api.technicpack.net/modpack/{modpack-name}

async fn handle_technic_search(request: TechnicSearchRequest) -> Result<TechnicSearchResult> {
    let url = format!("{}?q={}&offset={}&limit={}", 
        TECHNIC_SEARCH_URL,
        request.query.as_ref().unwrap_or(&"".into()),
        request.offset,
        request.limit);
    
    let response = http_client.get(&url).await?;
    let result = response.json::<TechnicSearchResult>().await?;
    Ok(result)
}

async fn handle_technic_modpack_info(request: TechnicModpackRequest) -> Result<TechnicModpackInfo> {
    let url = format!("{}/{}", TECHNIC_MODPACK_URL, request.name);
    let response = http_client.get(&url).await?;
    let result = response.json::<TechnicModpackInfo>().await?;
    Ok(result)
}
```

### 2. **Modpack Metadata Handler**

Location: `crates/backend/src/mod_metadata.rs`

Similar to how CurseForge and Modrinth modpacks are handled:

```rust
impl ModMetadataManager {
    fn load_technic_modpack<R: HasCursor>(
        &self, 
        hash: [u8; 20], 
        archive: &ArchiveHandle<R>,
        file: EntryHandle<R>
    ) -> Option<Arc<ContentSummary>> {
        // Parse manifest.json from archive
        // Extract modpack info: name, version, minecraft version
        // Parse file list for dependencies
        // Create ContentSummary with ContentType::TechnicModpack
        // Return with metadata
    }
}
```

### 3. **Install Content Logic**

Location: `crates/backend/src/install_content.rs`

Add handling for `ContentDownload::Technic`:

```rust
ContentDownload::Technic { modpack_name, version } => {
    // 1. Fetch modpack info from Technic API
    // 2. Get download URL for the specified version
    // 3. Download the modpack .zip file
    // 4. Extract and install contents
    // 5. Parse manifest to determine minecraft version
    // 6. Install all bundled mods
}
```

## Key Implementation Patterns

Learn from existing implementations:

1. **CurseForge Pattern** (crates/backend/src/install_content.rs:~530-600)
   - Shows how to fetch modpack manifest
   - Parse file dependencies
   - Handle mod installation

2. **Modrinth Pattern** (crates/backend/src/install_content.rs:~400-500)
   - Shows download URL handling
   - Dependency resolution
   - File extraction

## API Documentation

### Technic Platform API

**Search Modpacks:**
```
GET https://api.technicpack.net/search?q={query}
Response: { results: [{name, display_name, description, author, icon, ...}], total, ... }
```

**Get Modpack Info:**
```
GET https://api.technicpack.net/modpack/{modpack-name}
Response: { name, display_name, versions: [{version, minecraft, java, ...}], ... }
```

**Download Modpack:**
- Typically from: `https://launcher.technicpack.net/download/{modpack-name}/{version}`
- Returns a ZIP file containing modpack files and manifest.json

## Testing Checklist

- [ ] Backend compiles without errors
- [ ] Search returns Technic modpacks
- [ ] Clicking install opens the modal
- [ ] Selecting instance and version works
- [ ] Modpack downloads and installs
- [ ] Mods from modpack appear in instance
- [ ] Installed modpacks show as "Installed" in search

## Files Modified Summary

- `crates/schema/src/technic.rs` - NEW
- `crates/schema/src/lib.rs` - Added technic module
- `crates/schema/src/content.rs` - Added TechnicModpack variant
- `crates/bridge/src/install.rs` - Added ContentDownload::Technic
- `crates/bridge/src/meta.rs` - Added Technic metadata requests
- `crates/frontend/src/pages/technic_page.rs` - NEW
- `crates/frontend/src/modals/technic_install.rs` - NEW
- `crates/frontend/src/pages/mod.rs` - Added technic_page
- `crates/frontend/src/modals/mod.rs` - Added technic_install
- `crates/frontend/src/ui.rs` - Added Technic PageType
- `crates/frontend/src/pages/instance/mods_subpage.rs` - Added Technic button

## Notes for Completion

1. The frontend code is provided but may need adjustments based on your exact GPUI/component versions
2. Focus first on getting the backend metadata system working
3. Test with simple curl requests to Technic API before integrating
4. Consider error handling for network failures and invalid modpack data
5. Add proper progress tracking for modpack downloads
6. Consider implementing caching for API results to reduce server load

This should give you a solid foundation to complete the Technic Platform integration!
