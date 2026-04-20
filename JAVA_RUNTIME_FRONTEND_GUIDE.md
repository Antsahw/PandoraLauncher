# Java Runtime Management - Frontend Integration Guide

## Overview

This guide explains how to integrate Java runtime configuration UI into PandoraLauncher's frontend. The implementation uses GPUI (the same UI framework used throughout the application).

## Files Created

1. **java_runtime_config.rs** - UI components for Java runtime configuration
   - Location: `crates/frontend/src/components/java_runtime_config.rs`
   - Contains reusable panels for global, instance, and server configuration

2. **JAVA_RUNTIME_GUIDE.md** - User-facing documentation
   - Location: `PandoraLauncher/JAVA_RUNTIME_GUIDE.md`
   - Explains features, configuration files, and usage

## Integration Points

### 1. Global Settings Page
**File**: `crates/frontend/src/pages/settings/general.rs` (or similar)

**Integration**:
```rust
use crate::components::java_runtime_config::JavaRuntimesPanel;

// In your settings page render method:
pub fn render(&self, cx: &mut App) -> Div {
    div()
        .child(h1().text("Settings"))
        .child(JavaRuntimesPanel::new(&self.config.java_runtimes).render_content(cx))
}
```

**Features to add**:
- Display all configured Java runtimes
- Add/Edit/Delete runtimes
- Set global default runtime
- Validate Java executable paths
- Show availability status

### 2. Instance Settings Page
**File**: `crates/frontend/src/pages/instance/settings_subpage.rs`

**Integration**:
```rust
use crate::components::java_runtime_config::InstanceJavaRuntimePanel;

// Add to instance settings tabs:
pub fn render_java_runtime_tab(&self, cx: &mut App) -> Div {
    let available_runtimes = self.get_available_runtime_names();
    let panel = InstanceJavaRuntimePanel::new(
        available_runtimes,
        self.instance_config.java_runtime.as_ref(),
    );
    
    div()
        .child(h2().text("Java Runtime Configuration"))
        .child(panel.render_content(cx))
}
```

**Changes needed**:
- Add "Java Runtime" tab to instance settings
- When "Use Custom Runtime" is toggled, update `InstanceConfiguration::java_runtime`
- When runtime is selected, save to `info_v1.json`
- Show current runtime being used for the instance

### 3. Server Settings Page
**File**: `crates/frontend/src/pages/instance/server_settings_subpage.rs`

**Integration**:
```rust
use crate::components::java_runtime_config::ServerJavaConfigPanel;

// Add to server settings:
pub fn render_java_config(&self, cx: &mut App) -> Div {
    let available_runtimes = self.get_available_runtime_names();
    let panel = ServerJavaConfigPanel::new(
        available_runtimes,
        self.server_config.as_ref(),
    );
    
    div()
        .child(h2().text("Java Configuration"))
        .child(panel.render_content(cx))
        .child(h3().text("Memory"))
        .child(self.render_memory_sliders())
}
```

**Changes needed**:
- Add "Java Configuration" section to server settings
- Allow toggling custom runtime per server
- Configure min/max memory with sliders
- Save to `server_config.json` in server directory

## Backend API Usage

### Sending Backend Configuration Updates

```rust
// When user changes global Java runtime settings
send(MessageToBackend::UpdateBackendConfig {
    config: BackendConfig {
        java_runtimes: JavaRuntimesConfig {
            default_runtime: "java21".to_string(),
            runtimes: Some(BTreeMap::from([
                ("java21".to_string(), JavaRuntime {
                    name: "Java 21.0.1".to_string(),
                    path: "/usr/lib/jvm/java-21-openjdk/bin/java".to_string(),
                    version: 21,
                    available: true,
                }),
            ])),
        },
        // ... other fields
    },
    password: None,
});
```

### Updating Instance Configuration

```rust
// When user changes instance Java runtime
instance_config.java_runtime = Some(InstanceJavaRuntimeConfiguration {
    enabled: true,
    runtime_name: "java21".to_string(),
});
```

### Updating Server Configuration

```rust
// When user changes server Java runtime
server_config.java = Some(ServerJavaConfiguration {
    enabled: true,
    runtime_name: "java21".to_string(),
    memory: Some(ServerMemoryConfiguration {
        min: 512,
        max: 2048,
    }),
});
```

## Implementation Checklist

- [ ] Add `java_runtime_config.rs` to `crates/frontend/src/components/`
- [ ] Export new components in `crates/frontend/src/components/mod.rs`
- [ ] Add "Java Runtime" panel to global settings page
- [ ] Add "Java Runtime" tab to instance settings
- [ ] Add "Java Configuration" section to server settings page
- [ ] Implement file I/O for saving configurations:
  - [ ] Backend config: `backend_config.json`
  - [ ] Instance config: `info_v1.json`
  - [ ] Server config: `server_config.json`
- [ ] Add Java runtime detection utility (optional)
- [ ] Test with multiple Java versions installed
- [ ] Document in user guide

## Code Examples

### Getting Available Runtimes

```rust
fn get_available_runtime_names(&self) -> Vec<String> {
    let config = self.backend_config.read();
    config
        .java_runtimes
        .runtimes
        .as_ref()
        .map(|runtimes| runtimes.keys().cloned().collect())
        .unwrap_or_default()
}
```

### Loading Server Configuration

```rust
fn load_server_config(&self, server_name: &str) -> ServerConfiguration {
    let server_dir = self.launcher_dir.join("servers").join(server_name);
    let config_path = server_dir.join(".minecraft/server_config.json");
    
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        ServerConfiguration::default()
    }
}
```

### Saving Instance Configuration

```rust
fn save_instance_config(&self, instance_id: InstanceID, config: &InstanceConfiguration) {
    let instance = self.instances.get(instance_id).unwrap();
    let info_path = instance.dot_minecraft_folder.parent()
        .unwrap()
        .join("info_v1.json");
    
    let json = serde_json::to_string_pretty(&config).unwrap();
    std::fs::write(&info_path, json).unwrap();
}
```

## Testing

1. **Test Global Java Runtime**:
   - Create backend_config.json with multiple runtimes
   - Verify each runtime is available
   - Launch instance with each runtime
   - Confirm correct Java is used

2. **Test Instance Override**:
   - Set global default to Java 17
   - Override instance to use Java 21
   - Verify instance uses Java 21

3. **Test Server Configuration**:
   - Create server with custom runtime JSON
   - Launch server and verify Java path
   - Check memory settings in process

4. **Test Fallback Behavior**:
   - Set unavailable runtime name
   - Verify fallback to global default
   - Verify fallback to system Java

## Future Enhancements

1. **Auto-Detection**: Scan system for installed Java versions
2. **Download Management**: Download and manage specific Java versions
3. **Version Validation**: Check Java version matches instance requirements
4. **Environment Variables**: Allow custom JVM flags per runtime
5. **Performance Profiling**: Track memory usage and performance per runtime
