# Java Runtime Selection - Comprehensive Code Flow Investigation

## EXECUTIVE SUMMARY

- **Instances:** Java runtime IS checked, but has cascading fallback if not found in config map
- **Servers:** Java runtime COMPLETELY IGNORED due to SetServerJavaRuntime writing wrong JSON format
- **Discovery:** Runtimes ARE discovered at startup via `discover_downloaded_runtimes()`
- **Naming:** Runtime keys normalized to "java{version}" which may not match user's selection

---

## 1. INSTANCES: Load and Check Java Runtime

### 1.1 Frontend - User Selects Runtime

**File:** [crates/frontend/src/pages/instance/settings_subpage.rs](crates/frontend/src/pages/instance/settings_subpage.rs) (Line ~591)

```rust
// User selects runtime in UI
let java_runtime_configuration = InstanceJavaRuntimeConfiguration {
    enabled: true,
    runtime_name: "java25"  // User typed or selected this
};

// Send to backend
self.backend_handle.send(MessageToBackend::SetInstanceJavaRuntime {
    id: instance_id,
    java_runtime: java_runtime_configuration,
})
```

### 1.2 Backend Handler - Store Runtime Selection

**File:** [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs#L305)

```rust
MessageToBackend::SetInstanceJavaRuntime { id, java_runtime } => {
    if let Some(instance) = self.instance_state.write().instances.get_mut(&id) {
        instance.configuration.modify(|configuration| {
            configuration.java_runtime = Some(java_runtime);  // ✓ Stored in config
        });
    }
}
```

**Result:** Saved to disk in `instance.json`:
```json
{
    "java_runtime": {
        "enabled": true,
        "runtime_name": "java25"
    }
}
```

### 1.3 Launch Time - Check Java Runtime (THE KEY DECISION POINT)

**File:** [crates/backend/src/launch.rs](crates/backend/src/launch.rs#L878-L920)

```rust
async fn load_mojang_java_binary(
    ...
    configuration: &InstanceConfiguration,
    version_info: &MinecraftVersion,
    java_config: &schema::backend_config::JavaRuntimesConfig,  // Loaded from backend_config.json
    ...
) -> Result<PathBuf, LoadJavaRuntimeError> {
    
    // STEP 1: Check legacy jvm_binary setting
    if let Some(jvm_binary) = &configuration.jvm_binary {
        if jvm_binary.enabled && let Some(path) = &jvm_binary.path {
            if let Some(binary) = Self::search_for_java_binary(&path) {
                return Ok(binary);  // ✓ RETURN if found
            }
        }
    }

    // STEP 2: ★ CHECK PER-INSTANCE JAVA RUNTIME (NEW FEATURE)
    if let Some(java_runtime) = &configuration.java_runtime {
        if java_runtime.enabled && !java_runtime.runtime_name.is_empty() {
            
            // Call resolve_java_executable with the user's selected runtime name
            let java_path = crate::java_manager::resolve_java_executable(
                java_config, 
                Some(&java_runtime.runtime_name)  // "java25"
            );
            
            // If not "system" java, verify the path actually exists/works
            if java_runtime.runtime_name != "system" {
                if let Some(binary) = Self::search_for_java_binary(&java_path) {
                    return Ok(binary);  // ✓ RETURN if found
                }
                // ⚠️ If NOT found, falls through to next step
                // This is the "ignore and fallback" behavior
            } else {
                // "system" java - just return it
                return Ok(java_path);
            }
        }
    }

    // STEP 3: Check FORCE_EXTERNAL_JAVA environment variable
    if let Some(force_external_java) = std::env::var_os("FORCE_EXTERNAL_JAVA") {
        // Try to find matching Java version...
        // Returns if found, otherwise returns error
    }

    // STEP 4: ★ FALLBACK - Download Mojang Java (Version-Specific Default)
    let jre_component = if let Some(java_version) = &version_info.java_version {
        java_version.component  // e.g., "jre-21" for MC 1.20.1
    } else {
        "jre-legacy".into()     // Default for older MC versions
    };

    // Download and use Mojang runtime for this specific MC version
    let result = do_java_runtime_load(...).await;
    result
}
```

### 1.4 Java Runtime Resolution - Map Lookup

**File:** [crates/backend/src/java_manager.rs](crates/backend/src/java_manager.rs#L125-L170)

```rust
pub fn resolve_java_executable(
    java_config: &JavaRuntimesConfig,
    per_instance_runtime_name: Option<&str>,  // Some("java25")
) -> PathBuf {
    // Check per-instance override first
    if let Some(runtime_name) = per_instance_runtime_name {  // "java25"
        if !runtime_name.is_empty() && runtime_name != "system" {
            // Step A: Try to find "java25" in the runtimes map
            if let Some(runtime) = get_runtime_path(java_config, runtime_name) {
                return runtime;  // ✓ Found!
            }
            // ⚠️ NOT FOUND in map - continue to Step B
        } else if runtime_name == "system" {
            return PathBuf::from("java");  // System Java from PATH
        }
    }

    // Step B: Fall back to global default
    if !java_config.default_runtime.is_empty() && java_config.default_runtime != "system" {
        if let Some(runtime) = get_runtime_path(java_config, &java_config.default_runtime) {
            return runtime;
        }
    }

    // Step C: Final fallback - system Java from PATH
    PathBuf::from("java")
}

/// Internal lookup function
fn get_runtime_path(
    java_config: &JavaRuntimesConfig, 
    runtime_name: &str  // "java25"
) -> Option<PathBuf> {
    java_config
        .runtimes
        .as_ref()?
        .get(runtime_name)  // ⚠️ Exact key match required!
        .filter(|runtime| runtime.available)
        .map(|runtime| PathBuf::from(&runtime.path))
}
```

### 1.5 Cascade Flow (What Happens After Fallback)

```
User selects "java25"
    ↓
load_mojang_java_binary: calls resolve_java_executable("java25")
    ↓
resolve_java_executable: calls get_runtime_path("java25")
    ↓
get_runtime_path: java_config.runtimes.get("java25") → None ⚠️
    ↓
Falls back: Tries default_runtime
    ↓
Falls back: Returns PathBuf::from("java") (system Java)
    ↓
load_mojang_java_binary: search_for_java_binary("java") → Likely fails
    ↓
Falls back: Downloads Mojang Java for MC version
    ↓
Instance launches with Mojang Java, NOT user's selected runtime!
```

---

## 2. SERVERS: Java Runtime Completely Ignored (BUG!)

### 2.1 Frontend - User Selects Server Runtime

**File:** [crates/frontend/src/pages/instance/server_settings_subpage.rs](crates/frontend/src/pages/instance/server_settings_subpage.rs#L64)

```rust
self.backend_handle.send(bridge::message::MessageToBackend::SetServerJavaRuntime {
    name: server_name,
    java_runtime: "java25",
});
```

### 2.2 Backend Handler - WRITES WRONG JSON FORMAT (BUG!)

**File:** [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs#L233-L270)

```rust
MessageToBackend::SetServerJavaRuntime { name, java_runtime } => {
    log::info!("Setting server '{}' Java runtime to '{}'", name, java_runtime);
    
    // ... path setup ...
    
    // Load existing config or create new one
    let mut config = if config_path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap_or_default())
            .unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    
    // ❌ BUG: Writing flat structure instead of nested ServerJavaConfiguration
    config["java_runtime"] = serde_json::Value::String(java_runtime.to_string());
    
    // Writes this to server_config.json:
    // { "java_runtime": "java25" }
    
    if let Err(e) = std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap_or_default()) {
        self.send.send_error(format!("Failed to save server Java runtime: {}", e));
    } else {
        self.send.send_success(format!("Server Java runtime set to '{}'", java_runtime));
    }
}
```

### 2.3 Server Launch - Tries to Read Java Runtime

**File:** [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs#L2230-L2260)

```rust
// Load server configuration for Java settings
let server_config_path = server_dir.join(".minecraft").join("server_config.json");
let server_runtime_name: Option<String> = if server_config_path.exists() {
    if let Ok(content) = tokio::fs::read_to_string(&server_config_path).await {
        // ★ Tries to deserialize to ServerConfiguration struct
        if let Ok(config) = serde_json::from_str::<schema::server_config::ServerConfiguration>(&content) {
            // Expected structure:
            // {
            //   "java": {
            //     "enabled": true,
            //     "runtime_name": "java25"
            //   }
            // }
            //
            // But actually written by SetServerJavaRuntime handler:
            // {
            //   "java_runtime": "java25"
            // }
            
            config.java.and_then(|java| {
                if java.enabled && !java.runtime_name.is_empty() {
                    Some(java.runtime_name)
                } else {
                    None
                }
            })
            // ⚠️ Deserialization succeeds but config.java is None (field missing)
            // Returns: None
        } else {
            None
        }
    } else {
        None
    }
} else {
    None
};

// Resolve which Java to use
let java_config = self.config.write().get().java_runtimes.clone();
let java_executable = crate::java_manager::resolve_java_executable(
    &java_config,
    server_runtime_name.as_deref(),  // ⚠️ None (from above)
);
```

### 2.4 Result of Server Bug

```
User selects "java25" for server
    ↓
SetServerJavaRuntime writes: { "java_runtime": "java25" }
    ↓
Server launch reads config, expects: { "java": { "runtime_name": "java25" } }
    ↓
Deserialization: config.java = None (field doesn't exist!)
    ↓
resolve_java_executable(None) → Falls back through defaults
    ↓
Server always uses system Java!
```

---

## 3. JAVA RUNTIME DISCOVERY AT STARTUP

### 3.1 Backend Initialization

**File:** [crates/backend/src/backend.rs](crates/backend/src/backend.rs#L70-L85)

```rust
// Load configuration from disk
let mut config: Persistent<BackendConfig> = Persistent::load(directories.config_json.clone());

// ★ Discover any previously downloaded Java runtimes and populate config
config.modify(|cfg| {
    crate::java_manager::discover_downloaded_runtimes(
        &directories.runtime_base_dir,  // Path to runtimes directory
        &mut cfg.java_runtimes,         // JavaRuntimesConfig to populate
    );
});
```

### 3.2 Runtime Discovery Function

**File:** [crates/backend/src/java_manager.rs](crates/backend/src/java_manager.rs#L7-L76)

```rust
pub fn discover_downloaded_runtimes(
    runtime_base_dir: &Path,
    java_config: &mut JavaRuntimesConfig,
) {
    if !runtime_base_dir.exists() {
        return;
    }

    if let Ok(entries) = std::fs::read_dir(runtime_base_dir) {
        for entry in entries.flatten() {
            let jre_dir = entry.path();
            
            // Scan platform subdirectories (linux, mac-os-arm64, windows-x64, etc.)
            if let Ok(platform_entries) = std::fs::read_dir(&jre_dir) {
                for platform_entry in platform_entries.flatten() {
                    let platform_dir = platform_entry.path();
                    
                    // Look for Java binary at: {jre_dir}/{platform}/bin/java
                    let java_bin = platform_dir.join("bin/java");
                    if java_bin.exists() && java_bin.is_file() {
                        if let Ok(java_path) = java_bin.canonicalize() {
                            // Parse version from directory name (e.g., "java-21-openjdk" → 21)
                            if let Some(version) = parse_version_from_jre_name(&jre_name_str) {
                                // ★ Register with normalized key: "java21"
                                register_runtime(java_config, &jre_name_str, java_path, version);
                                break;
                            }
                        }
                    }
                    // ... also checks bin/javaw.exe (Windows) ...
                    // ... also checks jre.bundle (macOS) ...
                }
            }
        }
    }
}

fn register_runtime(
    java_config: &mut JavaRuntimesConfig,
    jre_name: &str,
    java_path: PathBuf,
    version: u32,
) {
    if java_config.runtimes.is_none() {
        java_config.runtimes = Some(BTreeMap::new());
    }

    if let Some(runtimes) = java_config.runtimes.as_mut() {
        // ★ NORMALIZED TO "java{version}" (e.g., "java21", "java25")
        let normalized_key = format!("java{}", version);
        
        // Only add if not already present (manual config takes priority)
        if !runtimes.contains_key(&normalized_key) {
            runtimes.insert(
                normalized_key,
                schema::backend_config::JavaRuntime {
                    name: format!("Java {}", version),
                    path: java_path.to_string_lossy().to_string(),
                    version,
                    available: true,
                },
            );
        }
    }
}
```

### 3.3 Runtime Name Extraction

```rust
fn parse_version_from_jre_name(name: &str) -> Option<u32> {
    // Examples: "java-21-openjdk", "java-17.0.1", "jre-11"
    for part in name.split(|c: char| c == '-' || c == '_' || c == '.') {
        if let Ok(num) = part.parse::<u32>() {
            if (8..=99).contains(&num) {
                // Likely a major Java version
                return Some(num);
            }
        }
    }
    None
}
```

### 3.4 Example Discovery Result

```
Discovered runtimes directory:
  runtimes/
    jre-legacy/
      linux/
        bin/java → Detected version 8 → Normalized to "java8"
    java-21-openjdk/
      linux/
        bin/java → Detected version 21 → Normalized to "java21"
    java-17-openjdk-aarch64/
      mac-os-arm64/
        bin/java → Detected version 17 → Normalized to "java17"

Added to JavaRuntimesConfig.runtimes:
{
    "java8": { path: ".../runtimes/jre-legacy/linux/bin/java", version: 8, available: true },
    "java21": { path: ".../runtimes/java-21-openjdk/linux/bin/java", version: 21, available: true },
    "java17": { path: ".../runtimes/java-17-openjdk-aarch64/mac-os-arm64/bin/java", version: 17, available: true },
}
```

---

## 4. FALLBACK BEHAVIOR IF JAVA25 ISN'T IN RUNTIMES MAP

### 4.1 Scenario: User selects "java25" but it's not in map

```
User's InstanceConfiguration:
{
    "java_runtime": {
        "enabled": true,
        "runtime_name": "java25"
    }
}

Backend Configuration (java_config):
{
    "runtimes": {
        "java21": { ... },  // Some discovered runtimes
        "java8": { ... },
        "system": { ... }
        // ⚠️ "java25" is NOT here!
    }
}
```

### 4.2 Resolution Call Flow

```
load_mojang_java_binary called
    ↓
resolve_java_executable(java_config, Some("java25"))
    ↓
get_runtime_path(java_config, "java25")
    ↓
java_config.runtimes.get("java25") → None ⚠️
    ↓
Returns None
    ↓
resolve_java_executable continues:
    Check config.default_runtime (e.g., "java21")
    ↓
    get_runtime_path(java_config, "java21")
    ↓
    java_config.runtimes.get("java21") → Some(runtime) ✓
    ↓
    Returns "/path/to/java21"
    ↓
Back to load_mojang_java_binary:
    search_for_java_binary("/path/to/java21") → Some(java_path) ✓
    Returns ✓
    ↓
Instance launches with Java 21, user wanted Java 25!
```

### 4.3 Why "java25" Might Not Be Found

**Reason 1:** Runtime directory name doesn't contain version
- Directory: `/runtimes/my-custom-jdk/`
- parse_version_from_jre_name("my-custom-jdk") → None
- Not registered at all!

**Reason 2:** Version parsing failed
- Directory: `/runtimes/jdk_with_no_version/`
- Couldn't extract version number
- Not registered

**Reason 3:** User typed different name
- User selected "java-25-openjdk" from somewhere
- But register_runtime normalized to "java25"
- Key mismatch!

**Reason 4:** Runtime hasn't been downloaded yet
- Config only has manually configured runtimes
- No discovery of system-installed Java runtimes
- "java25" simply doesn't exist on system

---

## 5. CODE FLOW DIAGRAM

```
INSTANCES:
┌─────────────────────────────────────────────────────────────┐
│ load_mojang_java_binary()                                   │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Check jvm_binary setting                               │
│     ↓                                                        │
│  2. Check java_runtime setting                             │
│     ↓                                                        │
│     resolve_java_executable("java25", JavaRuntimesConfig)  │
│     ↓                                                        │
│     get_runtime_path("java25") → ?                         │
│     ├─ FOUND: return /path/to/java25                       │
│     └─ NOT FOUND: continue                                  │
│     ↓                                                        │
│     Try default_runtime                                     │
│     ↓                                                        │
│     Try system Java                                         │
│                                                              │
│  3. Check FORCE_EXTERNAL_JAVA env                          │
│     ↓                                                        │
│  4. Download/use Mojang Java (MC-version specific)         │
│     ↓                                                        │
│  Return Java executable                                     │
└─────────────────────────────────────────────────────────────┘

SERVERS:
┌─────────────────────────────────────────────────────────────┐
│ SetServerJavaRuntime message                               │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Write to server_config.json:                              │
│  { "java_runtime": "java25" }  ❌ WRONG STRUCTURE         │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│ Server launch                                               │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Read from server_config.json                              │
│  Expect: { "java": { "runtime_name": "java25" } }         │
│  Got: { "java_runtime": "java25" }                         │
│  ↓                                                           │
│  Deserialize to ServerConfiguration                         │
│  ↓                                                           │
│  config.java = None (field missing!) ❌                    │
│  ↓                                                           │
│  resolve_java_executable(None)                            │
│  ↓                                                           │
│  Falls back to system Java                                  │
│                                                              │
└─────────────────────────────────────────────────────────────┘

STARTUP:
┌─────────────────────────────────────────────────────────────┐
│ Backend initialization                                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Load BackendConfig from backend_config.json               │
│  ↓                                                           │
│  discover_downloaded_runtimes(runtime_base_dir)            │
│  ↓                                                           │
│  Scan runtime directories                                   │
│  ↓                                                           │
│  Find Java binaries                                         │
│  ↓                                                           │
│  Parse versions from directory names                        │
│  ↓                                                           │
│  Register with normalized keys: "java21", "java17", etc.   │
│  ↓                                                           │
│  Update JavaRuntimesConfig.runtimes                         │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 6. CRITICAL ISSUES SUMMARY

| Issue | Component | Severity | Impact |
|-------|-----------|----------|--------|
| **SetServerJavaRuntime writes wrong JSON** | Server handler | 🔴 CRITICAL | Servers always use system Java |
| **Instance fallback silently ignores selection** | launch.rs | 🟠 HIGH | User's selection ignored, uses default Mojang |
| **Runtime key mismatch on discovery** | java_manager.rs | 🟠 HIGH | Discovered runtimes not found by exact key |
| **No validation feedback to user** | All | 🟡 MEDIUM | User doesn't know runtime wasn't found |
| **Mojang runtime not persisted** | launch.rs | 🟡 MEDIUM | Downloaded runtime re-verified every launch |

---

## 7. REQUIRED FIXES (Priority Order)

### CRITICAL FIX 1: Fix SetServerJavaRuntime JSON Structure

**File:** [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs#L263-L265)

```rust
// WRONG (current):
config["java_runtime"] = serde_json::Value::String(java_runtime.to_string());

// CORRECT:
config["java"] = serde_json::json!({
    "enabled": true,
    "runtime_name": java_runtime.to_string()
});
```

### HIGH FIX 2: Support Multiple Runtime Name Formats

When looking up runtime, try both:
- `"java25"` (normalized)
- `"java-25-openjdk"` (original discovered name)
- Configurable aliases

### HIGH FIX 3: Persist Downloaded Mojang Runtimes

After `do_java_runtime_load()` completes, add to `JavaRuntimesConfig`:
```rust
register_runtime(java_config, "jre-legacy", java_path, version);
```

### MEDIUM FIX 4: Add Validation and Logging

When java_runtime not found:
```rust
log::warn!("Selected Java runtime '{}' not found in config, falling back", runtime_name);
```
