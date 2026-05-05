use serde::{Deserialize, Serialize};

/// Configuration for Minecraft servers, stored in .minecraft/server_config.json
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfiguration {
    /// Java runtime to use for this server
    #[serde(default, deserialize_with = "crate::try_deserialize", skip_serializing_if = "is_default_java_configuration")]
    pub java: Option<ServerJavaConfiguration>,
}

impl Default for ServerConfiguration {
    fn default() -> Self {
        Self { java: None }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ServerJavaConfiguration {
    /// Whether to use a specific Java runtime for this server (overrides global default)
    pub enabled: bool,
    /// Name of the Java runtime to use (e.g., "java21", "system")
    /// If "system", uses system Java from PATH
    /// If empty string or not set, uses global default
    #[serde(default, skip_serializing_if = "crate::skip_if_default", deserialize_with = "crate::try_deserialize")]
    pub runtime_name: String,
    /// Memory configuration for the server
    #[serde(default, skip_serializing_if = "crate::skip_if_none")]
    pub memory: Option<ServerMemoryConfiguration>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct ServerMemoryConfiguration {
    /// Minimum heap size in MB
    pub min: u32,
    /// Maximum heap size in MB
    pub max: u32,
}

impl ServerMemoryConfiguration {
    pub const DEFAULT_MIN: u32 = 512;
    pub const DEFAULT_MAX: u32 = 1024;

    pub fn default_configured() -> Self {
        Self {
            min: Self::DEFAULT_MIN,
            max: Self::DEFAULT_MAX,
        }
    }
}

impl Default for ServerMemoryConfiguration {
    fn default() -> Self {
        Self::default_configured()
    }
}

fn is_default_java_configuration(config: &Option<ServerJavaConfiguration>) -> bool {
    if let Some(config) = config {
        !config.enabled 
            && config.runtime_name.is_empty()
            && config.memory.is_none()
    } else {
        true
    }
}

/// Statistics tracking for Minecraft servers
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ServerStats {
    /// Total uptime in seconds across all sessions
    pub total_uptime_secs: u64,
    /// Number of times the server has been started
    pub start_count: u64,
    /// Unix timestamp (milliseconds) of last start
    pub last_started_unix_ms: Option<i64>,
}

