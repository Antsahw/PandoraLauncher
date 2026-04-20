# Java Runtime Management - Implementation Guide

## Overview

PandoraLauncher now supports flexible Java runtime management with both global and per-instance/server configuration options. This replaces the need to use system Java with custom Java installations that can be managed entirely within the launcher.

## Features

### 1. **Global Java Runtimes Configuration**
- Define named Java runtimes (e.g., "java21", "java17", etc.)
- Each runtime stores:
  - Friendly name (e.g., "Java 21.0.1")
  - Full path to Java executable
  - Java version number
  - Availability status

### 2. **Instance-Level Java Runtime Selection**
- Each game instance can override the global Java runtime
- Options:
  - Use global default
  - Use specific named runtime
  - Use system Java (from PATH)
  
### 3. **Server-Level Java Runtime & Memory Configuration**
- Each server can:
  - Select its own Java runtime (independent of instances)
  - Configure instance-specific memory settings (min/max heap)

## Configuration Files

### Backend Configuration
**Location**: `~/.pandora_launcher/backend_config.json`

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
      }
    }
  }
}
```

### Instance Configuration
**Location**: `~/.pandora_launcher/instances/{instance_name}/.minecraft/info_v1.json`

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

### Server Configuration
**Location**: `~/.pandora_launcher/servers/{server_name}/.minecraft/server_config.json`

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

## Priority Order

### Instance Launch
1. Instance-specific Java runtime (if enabled)
2. Instance-specific Java version (forced_java_version in jvm_binary)
3. Custom JVM binary path (if set)
4. Global default runtime
5. Centralized runtime directory (java17/, java21/, etc.)
6. Mojang-managed Java runtime
7. System Java

### Server Launch
1. Server-specific Java runtime (if enabled)
2. Global default runtime
3. System Java

## Usage

### Setting Up Java Runtimes

#### Option 1: Detect Installed Runtimes (Future Feature)
- Use detection to automatically find installed Java versions
- Add them to the launcher

#### Option 2: Manual Configuration
- Edit `backend_config.json` directly
- Or use frontend UI (when implemented)

### Selecting Runtime for Instance

1. Open instance settings
2. Navigate to "Java Runtime" tab
3. Enable "Use Custom Java Runtime"
4. Select from available runtimes

### Selecting Runtime for Server

1. Edit `.minecraft/server_config.json` in server directory
2. Set desired runtime name and memory settings

## API Usage (For Frontend)

### Get/Set Backend Config
```rust
// Update backend config with new runtime
BackendConfigWithPassword {
  config: BackendConfig {
    java_runtimes: JavaRuntimesConfig {
      default_runtime: "java21".to_string(),
      runtimes: Some(HashMap::from([
        ("java21".to_string(), JavaRuntime { ... }),
      ])),
    },
    ...
  },
  password: None,
}
```

### Instance Settings
```rust
// In InstanceConfiguration
java_runtime: Some(InstanceJavaRuntimeConfiguration {
  enabled: true,
  runtime_name: "java21".to_string(),
})
```

## Future Enhancements

1. **Automatic Detection**: Scan system for installed Java versions
2. **Download Management**: Download specific Java versions from vendors
3. **UI Components**: Full frontend interface for:
   - Adding/editing Java runtimes
   - Per-instance runtime selection
   - Per-server runtime and memory configuration
4. **Version Validation**: Verify Java version matches instance requirements
5. **Performance Monitoring**: Track Java usage per instance/server
