use schema::backend_config::JavaRuntimesConfig;
use std::path::{Path, PathBuf};
use std::collections::BTreeMap;

/// Scans the runtime directory for previously downloaded Mojang Java runtimes
/// and populates the JavaRuntimesConfig with them
pub fn discover_downloaded_runtimes(
    runtime_base_dir: &Path,
    java_config: &mut JavaRuntimesConfig,
) {
    if !runtime_base_dir.exists() {
        return;
    }

    if let Ok(entries) = std::fs::read_dir(runtime_base_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if !metadata.is_dir() {
                    continue;
                }

                let jre_dir = entry.path();
                if let Some(jre_name) = jre_dir.file_name() {
                    let jre_name_str = jre_name.to_string_lossy().to_string();
                    
                    // Try to find the Java binary in platform subdirectories
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
                                        // Try to determine version from directory name first
                                        let mut version = parse_version_from_jre_name(&jre_name_str);
                                        
                                        // If not found in name, query the Java binary itself
                                        if version.is_none() {
                                            version = query_java_version(&java_bin);
                                        }
                                        
                                        if let Some(ver) = version {
                                            register_runtime(java_config, &jre_name_str, java_path, ver);
                                        }
                                        break;
                                    }
                                }
                                
                                // Check for javaw binary (Windows)
                                let javaw_bin = platform_dir.join("bin/javaw.exe");
                                if javaw_bin.exists() && javaw_bin.is_file() {
                                    if let Ok(javaw_path) = javaw_bin.canonicalize() {
                                        let mut version = parse_version_from_jre_name(&jre_name_str);
                                        if version.is_none() {
                                            version = query_java_version(&javaw_bin);
                                        }
                                        if let Some(ver) = version {
                                            register_runtime(java_config, &jre_name_str, javaw_path, ver);
                                        }
                                        break;
                                    }
                                }
                                
                                // Check for macOS bundle
                                let jre_bundle = platform_dir.join("jre.bundle/Contents/Home/bin/java");
                                if jre_bundle.exists() && jre_bundle.is_file() {
                                    if let Ok(jre_bundle_path) = jre_bundle.canonicalize() {
                                        let mut version = parse_version_from_jre_name(&jre_name_str);
                                        if version.is_none() {
                                            version = query_java_version(&jre_bundle);
                                        }
                                        if let Some(ver) = version {
                                            register_runtime(java_config, &jre_name_str, jre_bundle_path, ver);
                                        }
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

/// Register a discovered Java runtime in the config
fn register_runtime(
    java_config: &mut JavaRuntimesConfig,
    _jre_name: &str,
    java_path: PathBuf,
    version: u32,
) {
    // Ensure runtimes map exists
    if java_config.runtimes.is_none() {
        java_config.runtimes = Some(BTreeMap::new());
    }

    if let Some(runtimes) = java_config.runtimes.as_mut() {
        // Normalize the runtime name to "java{version}" format (e.g., "java25", "java21")
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

/// Try to parse Java version from JRE directory name
/// Examples: "java-21-openjdk", "java-17.0.1", "jre-11"
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

/// Query the Java binary to determine its version
/// Runs `java -version` and parses the output
fn query_java_version(java_path: &Path) -> Option<u32> {
    match std::process::Command::new(java_path)
        .arg("-version")
        .output()
    {
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let version_str = if !stderr.is_empty() { &stderr } else { &stdout };
            
            // Look for patterns like "version "21.0.7"" or "openjdk version "25.0.1""
            for word in version_str.split(|c: char| c.is_whitespace() || c == '"') {
                // Try to parse the word as a version number
                // Handle both "25.0.1" and "25" formats
                if let Some(dot_pos) = word.find('.') {
                    // Extract major version before the first dot
                    if let Ok(num) = word[..dot_pos].parse::<u32>() {
                        if (8..=99).contains(&num) {
                            return Some(num);
                        }
                    }
                } else if let Ok(num) = word.parse::<u32>() {
                    if (8..=99).contains(&num) {
                        return Some(num);
                    }
                }
            }
            None
        }
        Err(_) => None,
    }
}

/// Resolves which Java executable to use based on priority:
/// 1. Per-instance/server override (if enabled and non-empty)
/// 2. Global default (if set)
/// 3. System Java (fallback)
pub fn resolve_java_executable(
    java_config: &JavaRuntimesConfig,
    per_instance_runtime_name: Option<&str>,
) -> PathBuf {
    // Check per-instance override first
    if let Some(runtime_name) = per_instance_runtime_name {
        if !runtime_name.is_empty() && runtime_name != "system" {
            if let Some(runtime) = get_runtime_path(java_config, runtime_name) {
                return runtime;
            }
        } else if runtime_name == "system" {
            return PathBuf::from("java");
        }
    }

    // Fall back to global default
    if !java_config.default_runtime.is_empty() && java_config.default_runtime != "system" {
        if let Some(runtime) = get_runtime_path(java_config, &java_config.default_runtime) {
            return runtime;
        }
    }

    // Final fallback: system Java from PATH
    PathBuf::from("java")
}

/// Get the Java executable path for a specific runtime name
fn get_runtime_path(java_config: &JavaRuntimesConfig, runtime_name: &str) -> Option<PathBuf> {
    java_config
        .runtimes
        .as_ref()?
        .get(runtime_name)
        .filter(|runtime| runtime.available)
        .map(|runtime| PathBuf::from(&runtime.path))
}

/// Validate that a Java executable is working
#[allow(dead_code)]
pub fn validate_java_executable(java_path: &Path) -> bool {
    match std::process::Command::new(java_path)
        .arg("-version")
        .output()
    {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Detect system Java installations and merge them into the config
/// This adds system-detected Java runtimes without overriding manually configured ones
pub fn merge_system_java_runtimes(java_config: &mut JavaRuntimesConfig) {
    let system_runtimes = crate::java_runtime::detect_java_runtimes();
    
    // Ensure runtimes map exists
    if java_config.runtimes.is_none() {
        java_config.runtimes = Some(BTreeMap::new());
    }

    if let Some(runtimes) = java_config.runtimes.as_mut() {
        for (key, system_runtime) in system_runtimes {
            // Only add if not already present (manually configured runtimes take priority)
            if !runtimes.contains_key(&key) {
                runtimes.insert(key, system_runtime);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schema::backend_config::JavaRuntime;
    use std::collections::BTreeMap;

    #[test]
    fn test_resolve_java_per_instance_override() {
        let mut java_config = JavaRuntimesConfig::default();
        java_config.default_runtime = "java17".to_string();
        
        let mut runtimes = BTreeMap::new();
        runtimes.insert(
            "java21".to_string(),
            JavaRuntime {
                name: "Java 21".to_string(),
                path: "/usr/lib/jvm/java-21/bin/java".to_string(),
                version: 21,
                available: true,
            },
        );
        java_config.runtimes = Some(runtimes);

        let java_path = resolve_java_executable(&java_config, Some("java21"));
        assert_eq!(java_path.to_string_lossy(), "/usr/lib/jvm/java-21/bin/java");
    }

    #[test]
    fn test_resolve_java_global_default() {
        let mut java_config = JavaRuntimesConfig::default();
        java_config.default_runtime = "java17".to_string();
        
        let mut runtimes = BTreeMap::new();
        runtimes.insert(
            "java17".to_string(),
            JavaRuntime {
                name: "Java 17".to_string(),
                path: "/usr/lib/jvm/java-17/bin/java".to_string(),
                version: 17,
                available: true,
            },
        );
        java_config.runtimes = Some(runtimes);

        let java_path = resolve_java_executable(&java_config, None);
        assert_eq!(java_path.to_string_lossy(), "/usr/lib/jvm/java-17/bin/java");
    }

    #[test]
    fn test_resolve_java_system_fallback() {
        let java_config = JavaRuntimesConfig::default();
        let java_path = resolve_java_executable(&java_config, None);
        assert_eq!(java_path.to_string_lossy(), "java");
    }
}
