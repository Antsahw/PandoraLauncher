use serde::Deserialize;

pub mod assets_index;
pub mod auxiliary;
pub mod backend_config;
pub mod content;
pub mod curseforge;
pub mod fabric_launch;
pub mod fabric_loader_manifest;
pub mod fabric_mod;
pub mod forge;
pub mod forge_mod;
pub mod instance;
pub mod java_runtime_component;
pub mod java_runtimes;
pub mod loader;
pub mod maven;
pub mod minecraft_profile;
pub mod modification;
pub mod modrinth;
pub mod mrpack;
pub mod pandora_update;
pub mod resourcepack;
pub mod server_config;
pub mod server_software_versions;
pub mod technic;
pub mod text_component;
pub mod version;
pub mod version_manifest;

pub fn try_deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + Default,
    D: serde::Deserializer<'de>,
{
    Ok(T::deserialize(serde_json::Value::deserialize(deserializer)?).unwrap_or_default())
}

pub fn skip_if_default<T: Default + PartialEq>(value: &T) -> bool {
    value == &T::default()
}

pub fn skip_if_none<T>(value: &Option<T>) -> bool {
    value.is_none()
}

pub fn default_true() -> bool {
    true
}

pub fn single_or_seq<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    if let Ok(value) = T::deserialize(value.clone()) {
        Ok(vec![value])
    } else if let Ok(value) = <Vec<T>>::deserialize(value) {
        Ok(value)
    } else {
        Ok(Vec::new())
    }
}

/// Get the recommended Java version for a Minecraft version string
/// Based on Mojang's official Java version mappings
pub fn get_recommended_java_version(minecraft_version: &str) -> u32 {
    // Parse version components (e.g., "1.20.1" -> (1, 20, 1))
    let parts: Vec<&str> = minecraft_version.split('.').collect();
    
    if parts.len() < 2 {
        return 8; // Default to Java 8 for unknown versions
    }
    
    let major = parts[0].parse::<u32>().unwrap_or(1);
    let minor = parts[1].parse::<u32>().unwrap_or(0);
    
    match (major, minor) {
        // 1.0 - 1.11: Java 6/7 (but we use Java 8 as minimum)
        (1, 0..=11) => 8,
        // 1.12 - 1.16: Java 8
        (1, 12..=16) => 8,
        // 1.17 - 1.20: Java 16/17
        (1, 17..=20) => 17,
        // 1.21+: Java 21
        (1, 21..) => 21,
        // Future versions: use Java 21
        _ => 21,
    }
}
