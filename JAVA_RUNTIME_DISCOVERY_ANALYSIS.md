# Java Runtime Discovery and Resolution Analysis

## Overview
This document traces how Java runtimes are discovered, stored, and resolved when launching Minecraft. The issue: **user selects "java25" but the game launches with Java 21**.

---

## 1. Runtime Storage Location

**File:** [crates/backend/src/directories.rs](crates/backend/src/directories.rs#L50-L54)

```rust
let runtime_base_dir = launcher_dir.join("runtime");
let java_runtimes_dir = launcher_dir.join("java_runtimes");
```

**Two directories exist:**
- `${LAUNCHER_DIR}/runtime/` - Where Mojang-downloaded Java runtimes are stored
- `${LAUNCHER_DIR}/java_runtimes/` - Alternative Java runtimes directory

The discovery function scans `runtime_base_dir` (the "runtime" directory).

---

## 2. Runtime Discovery Function

**File:** [crates/backend/src/java_manager.rs](crates/backend/src/java_manager.rs#L7-L70)

### Function: `discover_downloaded_runtimes()`

```rust
pub fn discover_downloaded_runtimes(
    runtime_base_dir: &Path,
    java_config: &mut JavaRuntimesConfig,
) {
    if !runtime_base_dir.exists() {
        return;  // Exit if directory doesn't exist!
    }

    if let Ok(entries) = std::fs::read_dir(runtime_base_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if !metadata.is_dir() {
                    continue;  // Skip files, only look at directories
                }

                let jre_dir = entry.path();
                if let Some(jre_name) = jre_dir.file_name() {
                    let jre_name_str = jre_name.to_string_lossy().to_string();
                    
                    // Try to find Java binary in platform subdirectories
                    if let Ok(platform_entries) = std::fs::read_dir(&jre_dir) {
                        for platform_entry in platform_entries.flatten() {
                            if let Ok(platform_metadata) = platform_entry.metadata() {
                                if !platform_metadata.is_dir() {
                                    continue;
                                }

                                let platform_dir = platform_entry.path();
                                
                                // Check for java binary (Unix)
                                let java_bin = platform_dir.join("bin/java");
                                if java_bin.exists() && java_bin.is_file() {
                                    if let Ok(java_path) = java_bin.canonicalize() {
                                        let version = parse_version_from_jre_name(&jre_name_str).unwrap_or(0);
                                        register_runtime(java_config, &jre_name_str, java_path, version);
                                        break;
                                    }
                                }
                                
                                // Check for javaw binary (Windows)
                                let javaw_bin = platform_dir.join("bin/javaw.exe");
                                if javaw_bin.exists() && javaw_bin.is_file() {
                                    if let Ok(javaw_path) = javaw_bin.canonicalize() {
                                        let version = parse_version_from_jre_name(&jre_name_str).unwrap_or(0);
                                        register_runtime(java_config, &jre_name_str, javaw_path, version);
                                        break;
                                    }
                                }
                                
                                // Check for macOS bundle
                                let jre_bundle = platform_dir.join("jre.bundle/Contents/Home/bin/java");
                                if jre_bundle.exists() && jre_bundle.is_file() {
                                    if let Ok(jre_bundle_path) = jre_bundle.canonicalize() {
                                        let version = parse_version_from_jre_name(&jre_name_str).unwrap_or(0);
                                        register_runtime(java_config, &jre_name_str, jre_bundle_path, version);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
```

### Key Discovery Points:
1. **Directory structure expected:** `runtime/{jre_name}/{platform_dir}/bin/java`
   - Example: `runtime/java-21-openjdk/linux/bin/java`
2. **Names assigned:** Uses the directory name directly (e.g., "java-25", "java25", etc.)
3. **Version parsing:** Extracts version number from directory name using `parse_version_from_jre_name()`

### Version Parser:

```rust
fn parse_version_from_jre_name(name: &str) -> Option<u32> {
    // Split by common delimiters and look for version numbers
    for part in name.split(|c: char| c == '-' || c == '_' || c == '.') {
        if let Ok(num) = part.parse::<u32>() {
            if (8..=99).contains(&num) {
                // Likely a major Java version (8-99)
                return Some(num);
            }
        }
    }
    None
}
```

**Examples that would parse correctly:**
- `java-21-openjdk` → 21 ✓
- `java-25-openjdk` → 25 ✓
- `java25` → 25 ✓
- `jre-11` → 11 ✓

---

## 3. Runtime Registration

**File:** [crates/backend/src/java_manager.rs](crates/backend/src/java_manager.rs#L76-L100)

### Function: `register_runtime()`

```rust
fn register_runtime(
    java_config: &mut JavaRuntimesConfig,
    jre_name: &str,
    java_path: PathBuf,
    version: u32,
) {
    // Ensure runtimes map exists
    if java_config.runtimes.is_none() {
        java_config.runtimes = Some(BTreeMap::new());
    }

    if let Some(runtimes) = java_config.runtimes.as_mut() {
        // Only add if not already present (manual config takes priority)
        if !runtimes.contains_key(jre_name) {
            runtimes.insert(
                jre_name.to_string(),
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

**Important:** Manual config takes priority - discovered runtimes don't overwrite existing entries.

---

## 4. Discovery Called at Startup

**File:** [crates/backend/src/backend.rs](crates/backend/src/backend.rs#L78-L85)

```rust
let directories = Arc::new(LauncherDirectories::new(launcher_dir));

let mut config: Persistent<BackendConfig> = Persistent::load(directories.config_json.clone());

// Discover any previously downloaded Java runtimes and populate config
config.modify(|cfg| {
    crate::java_manager::discover_downloaded_runtimes(
        &directories.runtime_base_dir,
        &mut cfg.java_runtimes,
    );
});
```

This is called **once at startup** to populate discovered runtimes into the main config.

---

## 5. Runtime Resolution at Launch

**File:** [crates/backend/src/java_manager.rs](crates/backend/src/java_manager.rs#L122-L145)

### Function: `resolve_java_executable()`

```rust
pub fn resolve_java_executable(
    java_config: &JavaRuntimesConfig,
    per_instance_runtime_name: Option<&str>,
) -> PathBuf {
    // Check per-instance override first (HIGHEST PRIORITY)
    if let Some(runtime_name) = per_instance_runtime_name {
        if !runtime_name.is_empty() && runtime_name != "system" {
            if let Some(runtime) = get_runtime_path(java_config, runtime_name) {
                return runtime;  // Found the per-instance runtime
            }
            // If not found, fall through to next priority
        } else if runtime_name == "system" {
            return PathBuf::from("java");  // Use system Java from PATH
        }
    }

    // Fall back to global default (SECOND PRIORITY)
    if !java_config.default_runtime.is_empty() && java_config.default_runtime != "system" {
        if let Some(runtime) = get_runtime_path(java_config, &java_config.default_runtime) {
            return runtime;
        }
    }

    // Final fallback: system Java from PATH (LOWEST PRIORITY)
    PathBuf::from("java")
}
```

### Function: `get_runtime_path()`

```rust
fn get_runtime_path(java_config: &JavaRuntimesConfig, runtime_name: &str) -> Option<PathBuf> {
    java_config
        .runtimes
        .as_ref()?
        .get(runtime_name)           // Lookup runtime by name in map
        .filter(|runtime| runtime.available)  // Must be marked available
        .map(|runtime| PathBuf::from(&runtime.path))  // Return the stored path
}
```

---

## 6. Runtime Resolution During Launch

**File:** [crates/backend/src/launch.rs](crates/backend/src/launch.rs#L885-L920)

### Function: `load_mojang_java_binary()`

```rust
async fn load_mojang_java_binary(
    &self,
    meta: &MetadataManager,
    http_client: &reqwest::Client,
    configuration: &InstanceConfiguration,  // Per-instance config
    version_info: &MinecraftVersion,
    progress_trackers: &ProgressTrackers,
    launch_tracker: &ProgressTracker,
    java_config: &schema::backend_config::JavaRuntimesConfig,
) -> Result<PathBuf, LoadJavaRuntimeError> {
    // PRIORITY 1: Check per-instance custom jvm_binary
    if let Some(jvm_binary) = &configuration.jvm_binary {
        if jvm_binary.enabled && let Some(path) = &jvm_binary.path {
            if let Some(binary) = Self::search_for_java_binary(&path) {
                return Ok(binary);
            }
        }
    }

    // PRIORITY 2: Check per-instance java_runtime selection
    if let Some(java_runtime) = &configuration.java_runtime {
        if java_runtime.enabled && !java_runtime.runtime_name.is_empty() {
            // This is where "java25" would be used!
            let java_path = crate::java_manager::resolve_java_executable(java_config, Some(&java_runtime.runtime_name));
            
            // If not "system" java, verify the path actually exists/works
            if java_runtime.runtime_name != "system" {
                if let Some(binary) = Self::search_for_java_binary(&java_path) {
                    return Ok(binary);
                }
                // If the configured runtime isn't found, fall through to default behavior
                // ⚠️ THIS IS KEY: If "java25" isn't found, it continues below!
            } else {
                // "system" java - just return it
                return Ok(java_path);
            }
        }
    }

    // PRIORITY 3: FORCE_EXTERNAL_JAVA environment variable
    if let Some(force_external_java) = std::env::var_os("FORCE_EXTERNAL_JAVA") {
        // ... looks for specific Java version ...
    }

    // PRIORITY 4: Load Mojang Java runtime components
    // ... rest of function attempts to download the required Java ...
}
```

### Key Issue Point:
If user selects "java25" but it's not in the `runtimes` map, the code **falls through** and tries to use the Mojang Java runtime for the Minecraft version instead (which defaults to Java 21 for many modern versions).

---

## 7. Data Structures

**File:** [crates/schema/src/backend_config.rs](crates/schema/src/backend_config.rs#L18-L50)

```rust
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct JavaRuntimesConfig {
    /// Default Java runtime name to use globally (e.g., "java21", "system")
    pub default_runtime: String,
    
    /// Map of runtime names to their configurations
    /// Example: {"java21": JavaRuntime {...}, "java17": JavaRuntime {...}}
    pub runtimes: Option<BTreeMap<String, JavaRuntime>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JavaRuntime {
    /// Friendly name for display (e.g., "Java 21.0.1")
    pub name: String,
    
    /// Full path to Java executable
    pub path: String,
    
    /// Java version (8, 11, 17, 21, etc.)
    pub version: u32,
    
    /// Whether this runtime is available/valid
    pub available: bool,
}
```

**File:** [crates/schema/src/instance.rs](crates/schema/src/instance.rs#L136-L145)

```rust
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct InstanceJavaRuntimeConfiguration {
    /// Whether to use a specific Java runtime for this instance (overrides global default)
    pub enabled: bool,
    
    /// Name of the Java runtime to use (e.g., "java21", "system")
    pub runtime_name: String,
}
```

---

## Summary of the Issue

### The Problem: User Selects "java25" but Gets Java 21

**Possible Root Causes:**

1. **Java 25 Runtime Not Discovered**
   - Path: `${LAUNCHER_DIR}/runtime/` directory doesn't exist or is empty
   - No subdirectory with "java25" in the name
   - Or the binary at `runtime/java-25-xxx/{platform}/bin/java` doesn't exist
   
2. **Java 25 Not Registered in Config**
   - Discovery didn't run or failed
   - The runtimes map doesn't have an entry for "java25"
   
3. **Runtime Name Mismatch**
   - User selected name (e.g., "java25") doesn't match discovered name (e.g., "java-25-openjdk")
   - The lookup in `get_runtime_path()` fails because key doesn't exist
   
4. **Binary Not Found**
   - `resolve_java_executable()` returns a path for "java25"
   - But `search_for_java_binary()` can't locate the actual binary file
   - Code falls through to default Mojang Java (Java 21)

### Debug Steps:

1. **Check if runtime directory exists:**
   ```bash
   ls -la ~/.launcher-dir/runtime/
   ```

2. **Check what's discovered at startup:**
   - Look for log messages about discovered Java runtimes
   
3. **Check config.json:**
   ```bash
   cat ~/.launcher-dir/config.json | jq '.java_runtimes'
   ```
   Should show something like:
   ```json
   {
     "default_runtime": "java25",
     "runtimes": {
       "java-25-openjdk": {
         "name": "Java 25",
         "path": "/path/to/java",
         "version": 25,
         "available": true
       }
     }
   }
   ```

4. **Check instance configuration:**
   ```bash
   cat ~/.launcher-dir/instances/{instance-id}/info.json | jq '.java_runtime'
   ```
   Should show:
   ```json
   {
     "enabled": true,
     "runtime_name": "java25"  // or whatever the key is in config
   }
   ```

5. **Verify binary exists:**
   ```bash
   which java25
   ls -la ~/.launcher-dir/runtime/java-25-openjdk/linux/bin/java
   ```

### Most Likely Issue:
The instance config has `"runtime_name": "java25"`, but the runtimes map has the key `"java-25-openjdk"` (discovered name), so `get_runtime_path()` returns `None`, and the code falls through to use the default Mojang Java.

---

## Resolution Flowchart

```
User launches instance with "java25" selected
          ↓
load_mojang_java_binary() called
          ↓
Check per-instance jvm_binary setting (PRIORITY 1)
  - Not set or disabled → continue
  - Set and valid → use it
          ↓
Check per-instance java_runtime selection (PRIORITY 2)
  - "java25" found in request
  - Call: resolve_java_executable(java_config, Some("java25"))
          ↓
resolve_java_executable() → get_runtime_path(java_config, "java25")
          ↓
Lookup "java25" in java_config.runtimes map
  ✓ Found → return path       → Binary located via search_for_java_binary() → Use Java 25
  ✗ Not found → return None  → Fall through
          ↓
Fall through to PRIORITY 3: Mojang Java runtime
          ↓
Download/use Java 21 (default for Minecraft 1.20+)
```
