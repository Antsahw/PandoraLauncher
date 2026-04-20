# Java Runtime Investigation - Quick Reference

## Your Questions Answered

### 1. **In launch.rs load_mojang_java_binary() - what happens after java_runtime check fails?**

✓ **Java runtime IS checked** (Line 903-920)
- Calls `resolve_java_executable(java_config, Some(&java_runtime.runtime_name))`
- If runtime path not found in config map → **falls through to next check**
- Next checks: `FORCE_EXTERNAL_JAVA` env var
- Final fallback: **Downloads and uses Mojang Java runtime for the MC version**

**Code path after failure:**
```
resolve_java_executable returns "system java" default
  → search_for_java_binary("java") likely fails
  → Falls through to line 925: FORCE_EXTERNAL_JAVA check
  → Falls through to line 951: Download Mojang Java
  → Launches with Mojang runtime, NOT user's selected runtime
```

---

### 2. **In backend_handler.rs where servers launch - is the SetServerJavaRuntime message being handled?**

❌ **YES, but it's BROKEN!** (Line 233-270)

The handler exists BUT writes the wrong JSON structure:

```rust
// WRONG - What's currently written:
{ "java_runtime": "java25" }

// CORRECT - What should be written:
{
  "java": {
    "enabled": true,
    "runtime_name": "java25"
  }
}
```

**Result:** When server launches, it can't find the `java` field → defaults to system Java

---

### 3. **How is server_runtime_name passed/extracted when launching a server?**

When launching: [backend_handler.rs Lines 2230-2260]

```rust
let server_runtime_name: Option<String> = if server_config_path.exists() {
    if let Ok(config) = serde_json::from_str::<ServerConfiguration>(&content) {
        // Expects: config.java.runtime_name
        config.java.and_then(|java| {
            if java.enabled && !java.runtime_name.is_empty() {
                Some(java.runtime_name)  // Extracts the runtime name
            } else {
                None
            }
        })
    } else {
        None
    }
} else {
    None
};
```

**Issue:** SetServerJavaRuntime writes flat structure → config.java is None → server_runtime_name is None → Uses system Java

---

### 4. **Are java runtimes being discovered at startup?**

✓ **YES!** [backend.rs Lines 78-81]

```rust
config.modify(|cfg| {
    crate::java_manager::discover_downloaded_runtimes(
        &directories.runtime_base_dir,
        &mut cfg.java_runtimes,
    );
});
```

**Discovery process:**
1. Scans `{launcher_dir}/runtimes/` directory
2. Finds Java binaries in platform subdirectories
3. Parses version from directory name (e.g., "java-21-openjdk" → 21)
4. Registers with normalized key: `"java21"`, `"java17"`, etc.

**Discovered runtimes are added to** `JavaRuntimesConfig.runtimes` map

---

### 5. **What's the fallback behavior if java25 isn't in the runtimes map?**

If user selects "java25" but it's not in the config map:

```
1. resolve_java_executable("java25")
   → get_runtime_path("java25") returns None
   
2. Falls back to global default_runtime
   → get_runtime_path(default_runtime) 
   → If found: use it ✓
   → If not found: continue
   
3. Falls back to system Java
   → PathBuf::from("java")
   
4. (Instances only) Falls back to Mojang Java
   → Downloads MC-version-specific runtime
```

So the priority is:
1. **Per-instance/server override** (java25)
2. **Global default** (e.g., java21)
3. **System Java** ("java" from PATH)
4. **(Instances only) Mojang Java** (MC-version specific)

---

## Root Causes

| Problem | Root Cause | Location |
|---------|-----------|----------|
| **Servers ignore Java selection** | SetServerJavaRuntime writes wrong JSON | [backend_handler.rs:233](crates/backend/src/backend_handler.rs#L233) |
| **Instances fall back to Mojang** | Runtime name not in config map | [java_manager.rs:157](crates/backend/src/java_manager.rs#L157) |
| **Can't find discovered runtime** | Normalized to "java25" but user expects different name | [java_manager.rs:99](crates/backend/src/java_manager.rs#L99) |
| **No warning to user** | Silent fallback with no logging | [launch.rs:915](crates/backend/src/launch.rs#L915) |

---

## Code Flow Diagram

### Instance Launch
```
User selects "java25" in UI
        ↓
Frontend sends SetInstanceJavaRuntime
        ↓
Backend stores in instance.json
        ↓
User launches game
        ↓
load_mojang_java_binary()
        ├─ Check jvm_binary → not found
        ├─ Check java_runtime → "java25"
        │  ├─ resolve_java_executable("java25")
        │  │  ├─ get_runtime_path("java25") → None ⚠️ (not in map)
        │  │  ├─ Try default_runtime → Success/Fail
        │  │  └─ Try system Java → PathBuf::from("java")
        │  └─ search_for_java_binary → Likely fails
        ├─ Check FORCE_EXTERNAL_JAVA → not set
        └─ Download Mojang Java for MC version ✓
        ↓
Game launches with Mojang Java (user wanted java25!)
```

### Server Launch
```
User selects "java25" for server
        ↓
Frontend sends SetServerJavaRuntime("java25")
        ↓
Backend writes to server_config.json:
        { "java_runtime": "java25" } ❌ WRONG!
        ↓
User starts server
        ↓
Backend reads server_config.json
        ├─ Deserialize to ServerConfiguration
        ├─ Expects: config.java.runtime_name
        ├─ Got: none (field doesn't exist!)
        └─ server_runtime_name = None
        ↓
resolve_java_executable(None)
        └─ Falls back to default → Falls back to system Java
        ↓
Server launches with system Java!
```

---

## The CRITICAL BUG

**SetServerJavaRuntime writes:** `{ "java_runtime": "java25" }`

**Server launch expects:** `{ "java": { "enabled": true, "runtime_name": "java25" } }`

**Result:** Deserialization succeeds but the nested `java` field is missing, so `config.java` is `None`

This one line fix would solve the server Java runtime issue:

```diff
- config["java_runtime"] = serde_json::Value::String(java_runtime.to_string());
+ config["java"] = serde_json::json!({
+     "enabled": true,
+     "runtime_name": java_runtime.to_string()
+ });
```

---

## References

**Main Investigation Document:** See [JAVA_RUNTIME_INVESTIGATION.md](JAVA_RUNTIME_INVESTIGATION.md) for full code flows

**Key Files:**
- Instance Runtime Selection: [crates/backend/src/launch.rs](crates/backend/src/launch.rs#L878)
- Server Runtime Handler: [crates/backend/src/backend_handler.rs](crates/backend/src/backend_handler.rs#L233)
- Java Resolution: [crates/backend/src/java_manager.rs](crates/backend/src/java_manager.rs#L125)
- Runtime Discovery: [crates/backend/src/backend.rs](crates/backend/src/backend.rs#L78)
- Server Config Schema: [crates/schema/src/server_config.rs](crates/schema/src/server_config.rs)
