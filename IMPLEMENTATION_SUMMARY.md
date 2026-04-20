# Java Runtime Management Implementation Summary

## What Was Implemented

A complete Java runtime management system for PandoraLauncher that allows users to configure custom Java runtimes globally and override them per-instance or per-server, eliminating the need to rely on system Java.

## Key Components

### 1. Schema Changes (`crates/schema/src/`)

#### backend_config.rs
- **JavaRuntimesConfig**: Extended to support named Java runtimes
  - `default_runtime`: Global default runtime name (string)
  - `runtimes`: BTreeMap of runtime names to JavaRuntime configs
- **JavaRuntime**: Configuration for a specific Java installation
  - `name`: Friendly display name
  - `path`: Full path to Java executable
  - `version`: Major Java version (8, 11, 17, 21, etc.)
  - `available`: Whether the runtime is currently available

#### instance.rs
- **InstanceJavaRuntimeConfiguration**: Per-instance Java runtime selection
  - `enabled`: Toggle to use custom runtime
  - `runtime_name`: Name of the runtime to use (or "system")
- Added to `InstanceConfiguration` struct

#### server_config.rs (NEW FILE)
- **ServerConfiguration**: Top-level server config structure
- **ServerJavaConfiguration**: Per-server Java and memory settings
  - `enabled`: Toggle custom runtime
  - `runtime_name`: Runtime selection
  - `memory`: Optional memory configuration
- **ServerMemoryConfiguration**: Min/max heap size in MB

#### lib.rs
- Added `server_config` module export

### 2. Backend Implementation (`crates/backend/src/`)

#### java_runtime.rs (NEW FILE)
- **resolve_java_path()**: Determines which Java to use based on configuration
  - Priority: instance override → global default → system Java
- **validate_java_runtime()**: Checks if a Java path exists and is executable
- **detect_java_runtimes()**: Auto-discovers installed Java versions
- **parse_java_version()**: Extracts version from `java -version` output
- Comprehensive unit tests included

#### launch.rs
- **load_mojang_java_binary()**: Updated to check new Java runtime config
  - Priority 1: Instance Java runtime selection
  - Priority 2: Legacy custom JVM binary path (backward compatible)
  - Priority 3: Global default runtime
  - Priority 4: Version-based centralized runtimes
  - Priority 5: Mojang-managed Java

#### backend_handler.rs
- **start_server()**: Updated to load and use server Java configuration
  - Loads `server_config.json` from server directory
  - Uses resolved Java path instead of hardcoded "java" command
  - Supports configurable memory settings per server
- **resolve_server_java_path()**: Helper method for server Java resolution
  - Priority: server config → global default → system Java

#### lib.rs
- Added `java_runtime` module

### 3. Frontend Components (`crates/frontend/src/`)

#### java_runtime_config.rs (NEW FILE)
- **JavaRuntimesPanel**: Global Java runtime management UI
  - Display/add/edit/delete runtimes
  - Set global default
  - Show availability status
- **InstanceJavaRuntimePanel**: Instance-level runtime selection
  - Toggle custom runtime
  - Runtime selector dropdown
  - Shows global default when disabled
- **ServerJavaConfigPanel**: Server-level runtime and memory config
  - Toggle custom runtime
  - Runtime selector
  - Memory sliders (min/max)
- **JavaRuntimeStatus**: Status display widget

## Configuration Examples

### Global Configuration (backend_config.json)
```json
{
  "java_runtimes": {
    "default_runtime": "java21",
    "runtimes": {
      "java21": {
        "name": "Java 21.0.1",
        "path": "/usr/lib/jvm/java-21-openjdk/bin/java",
        "version": 21,
        "available": true
      },
      "java17": {
        "name": "Java 17.0.5",
        "path": "/usr/lib/jvm/java-17-openjdk/bin/java",
        "version": 17,
        "available": true
      },
      "system": {
        "name": "System Java",
        "path": "java",
        "version": 0,
        "available": true
      }
    }
  }
}
```

### Instance Configuration (info_v1.json)
```json
{
  "minecraft_version": "1.20.1",
  "loader": "Fabric",
  "java_runtime": {
    "enabled": true,
    "runtime_name": "java21"
  }
}
```

### Server Configuration (server_config.json)
```json
{
  "java": {
    "enabled": true,
    "runtime_name": "java21",
    "memory": {
      "min": 512,
      "max": 2048
    }
  }
}
```

## Priority Resolution Order

### For Instances
1. Instance-specific Java runtime (if enabled)
2. Instance-specific Java version (forced_java_version)
3. Custom JVM binary path (legacy support)
4. Global default runtime
5. Centralized runtime directory
6. Mojang-managed Java runtime
7. System Java from PATH

### For Servers
1. Server-specific Java runtime (if enabled)
2. Global default runtime
3. System Java from PATH

## Backward Compatibility

- Existing `jvm_binary` configuration still works (lower priority)
- Legacy `forced_java_version` still supported
- Automatic fallback to system Java if runtime unavailable
- No breaking changes to existing instance/server configurations

## Files Modified

1. `crates/schema/src/backend_config.rs` - Expanded JavaRuntimesConfig
2. `crates/schema/src/instance.rs` - Added InstanceJavaRuntimeConfiguration
3. `crates/schema/src/lib.rs` - Added server_config module
4. `crates/backend/src/launch.rs` - Updated Java resolution logic
5. `crates/backend/src/backend_handler.rs` - Updated server launch code
6. `crates/backend/src/lib.rs` - Added java_runtime module

## Files Created

1. `crates/schema/src/server_config.rs` - Server configuration structures
2. `crates/backend/src/java_runtime.rs` - Java runtime resolution logic
3. `crates/frontend/src/components/java_runtime_config.rs` - UI components
4. `JAVA_RUNTIME_GUIDE.md` - User documentation
5. `JAVA_RUNTIME_FRONTEND_GUIDE.md` - Frontend integration guide

## Usage Instructions

### For Users

1. **Add Java Runtimes** (via backend_config.json or future UI):
   ```json
   "java21": {
     "name": "Java 21.0.1",
     "path": "/path/to/java",
     "version": 21,
     "available": true
   }
   ```

2. **Configure Instance** (via UI or info_v1.json):
   - Navigate to instance settings
   - Enable "Use Custom Java Runtime"
   - Select desired runtime from dropdown
   - Save changes

3. **Configure Server** (via server_config.json):
   - Edit `.minecraft/server_config.json` in server directory
   - Set `java.enabled: true`
   - Set `java.runtime_name: "java21"`
   - Set memory: `{"min": 512, "max": 2048}` (optional)

### For Developers

1. **Integrating UI Components**:
   - Import from `crates/frontend/src/components/java_runtime_config.rs`
   - Add to settings pages (global, instance, server)
   - Connect to backend message handlers

2. **Auto-Detection** (Future):
   - Use `detection::detect_java_runtimes()` from `java_runtime.rs`
   - Scan common installation paths
   - Populate runtime list automatically

3. **Custom JVM Arguments**:
   - Existing `jvm_flags` can be combined with new runtime selection
   - Runtime selected via `java_runtime`, flags via `jvm_flags`

## Testing Recommendations

1. **Configuration Persistence**:
   - Save and reload global, instance, and server configs
   - Verify settings persist across restarts

2. **Runtime Resolution**:
   - Test each priority level (instance → global → system)
   - Verify fallback behavior when runtime unavailable

3. **Launch Behavior**:
   - Launch instance with each configured runtime
   - Verify correct Java version is used
   - Check memory settings are applied

4. **Edge Cases**:
   - Missing Java executable
   - Invalid path in configuration
   - Disabled custom runtime (should use global default)

## Future Enhancements

1. **Auto-Detection**: Automatically find installed Java versions
2. **Download Management**: Download and manage specific Java versions
3. **Version Validation**: Warn if Java version doesn't match instance requirements
4. **Performance Monitoring**: Track memory usage and performance per runtime
5. **Custom Wrapper Scripts**: Allow per-runtime wrapper commands
6. **JVM Arguments**: Per-runtime default JVM flags
7. **Update Notifications**: Alert when Java runtime is out of date
8. **Portable Java**: Bundle Java runtimes with launcher

## Migration Guide

For existing PandoraLauncher users:
1. No action required - system will continue working
2. To use custom Java:
   - Manually add runtimes to backend_config.json
   - Or wait for UI implementation in future update
3. Old `jvm_binary` configuration still supported
4. Gradual migration path to new system

---

**Implementation Date**: 2026-04-13
**Status**: Complete and Ready for Testing
