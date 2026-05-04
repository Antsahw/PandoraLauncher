use std::{path::PathBuf, collections::BTreeMap};
use schema::backend_config::JavaRuntime;

/// Resolves the Java executable path based on configuration
/// 
/// Priority order:
/// 1. If instance_runtime is enabled, use that runtime name
/// 2. If instance_runtime.runtime_name is "system", use system Java
/// 3. Use global default runtime from backend config
/// 4. Fall back to system Java if nothing configured
#[allow(dead_code)]
pub fn resolve_java_path(
    instance_runtime_name: Option<&str>,
    instance_runtime_enabled: bool,
    global_default: &str,
    available_runtimes: &BTreeMap<String, JavaRuntime>,
) -> PathBuf {
    // Check instance override first
    if instance_runtime_enabled {
        if let Some(runtime_name) = instance_runtime_name {
            if runtime_name == "system" {
                return get_system_java();
            }
            if let Some(runtime) = available_runtimes.get(runtime_name) {
                if runtime.available {
                    return PathBuf::from(&runtime.path);
                }
            }
        }
    }

    // Fall back to global default
    if !global_default.is_empty() && global_default != "system" {
        if let Some(runtime) = available_runtimes.get(global_default) {
            if runtime.available {
                return PathBuf::from(&runtime.path);
            }
        }
    }

    // Final fallback: system Java
    get_system_java()
}

/// Gets the system Java executable from PATH
fn get_system_java() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from("java.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("java")
    }
}

/// Validates that a Java runtime exists and is executable
#[allow(dead_code)]
pub fn validate_java_runtime(path: &str) -> bool {
    let path = PathBuf::from(path);
    path.exists() && path.is_file()
}

/// Detects installed Java runtimes on the system
/// 
/// This searches common installation paths and returns available runtimes
#[allow(dead_code)]
pub fn detect_java_runtimes() -> BTreeMap<String, JavaRuntime> {
    let mut runtimes = BTreeMap::new();

    #[cfg(unix)]
    {
        // Check common Unix Java installation paths
        let search_paths = vec![
            "/usr/lib/jvm/",
            "/usr/local/lib/jvm/",
            "/opt/jdk/",
            "/opt/openjdk/",
            #[cfg(target_os = "macos")]
            "/Library/Java/JavaVirtualMachines/",
            #[cfg(target_os = "macos")]
            "/usr/libexec/java_home",
        ];

        for base_path in search_paths {
            if let Ok(entries) = std::fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(java_bin) = find_java_binary(&path) {
                        if let Some(runtime) = create_runtime_from_path(&java_bin) {
                            runtimes.insert(runtime.name.to_lowercase().replace(" ", "_"), runtime);
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Check common Windows Java installation paths
        let base_paths = vec![
            format!(r"C:\Program Files\Java"),
            format!(r"C:\Program Files (x86)\Java"),
            std::env::var("JAVA_HOME").unwrap_or_default(),
        ];

        for base_path in base_paths.into_iter().filter(|p| !p.is_empty()) {
            if let Ok(entries) = std::fs::read_dir(&base_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(java_bin) = find_java_binary(&path) {
                        if let Some(runtime) = create_runtime_from_path(&java_bin) {
                            runtimes.insert(runtime.name.to_lowercase().replace(" ", "_"), runtime);
                        }
                    }
                }
            }
        }
    }

    runtimes
}

/// Finds java binary in a directory (recursively searches bin/)
#[allow(dead_code)]
fn find_java_binary(dir: &std::path::Path) -> Option<PathBuf> {
    // Look for bin/java or bin/java.exe
    let bin_dir = dir.join("bin");
    
    #[cfg(target_os = "windows")]
    {
        let java_exe = bin_dir.join("java.exe");
        if java_exe.exists() {
            return Some(java_exe);
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        let java = bin_dir.join("java");
        if java.exists() {
            return Some(java);
        }
    }
    
    None
}

/// Creates a JavaRuntime from a java binary path by detecting version
#[allow(dead_code)]
fn create_runtime_from_path(java_path: &std::path::Path) -> Option<JavaRuntime> {
    // Try to get version from `java -version`
    if let Ok(output) = std::process::Command::new(java_path)
        .arg("-version")
        .output()
    {
        let version_output = String::from_utf8_lossy(&output.stderr);
        if let Some(version) = parse_java_version(&version_output) {
            let friendly_name = format!("Java {}", version);
            return Some(JavaRuntime {
                name: friendly_name,
                path: java_path.to_string_lossy().to_string(),
                version,
                available: true,
            });
        }
    }

    None
}

/// Parses Java version from `java -version` output
#[allow(dead_code)]
fn parse_java_version(version_str: &str) -> Option<u32> {
    // Parse version like:
    // "openjdk version \"21.0.1\" 2023-10-17"
    // "java version \"1.8.0_392\""
    // Returns major version (21, 8, 17, etc.)
    
    // Try to find version number patterns
    if let Some(start) = version_str.find("\"") {
        let version_part = &version_str[start + 1..];
        if let Some(end) = version_part.find("\"") {
            let version_str = &version_part[..end];
            
            // Handle versions like "21.0.1"
            if let Some(dot_pos) = version_str.find(".") {
                let major = version_str[..dot_pos].parse::<u32>().ok();
                if major.is_some() && major.unwrap() > 1 {
                    return major;
                }
            }
            
            // Handle versions like "1.8.0_392" -> return 8
            if version_str.starts_with("1.") {
                if let Some(minor_end) = version_str[2..].find(".") {
                    return version_str[2..2 + minor_end].parse::<u32>().ok();
                }
            }
            
            // Try parsing just the first number
            if let Some(space_pos) = version_str.find(|c: char| !c.is_numeric()) {
                return version_str[..space_pos].parse::<u32>().ok();
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_java_version() {
        assert_eq!(parse_java_version("openjdk version \"21.0.1\" 2023-10-17"), Some(21));
        assert_eq!(parse_java_version("java version \"1.8.0_392\""), Some(8));
        assert_eq!(parse_java_version("java version \"11.0.19\""), Some(11));
        assert_eq!(parse_java_version("java version \"17.0.5\""), Some(17));
    }

    #[test]
    fn test_resolve_java_path() {
        let mut runtimes = BTreeMap::new();
        runtimes.insert("java21".to_string(), JavaRuntime {
            name: "Java 21".to_string(),
            path: "/usr/lib/jvm/java-21/bin/java".to_string(),
            version: 21,
            available: true,
        });

        // Instance override takes priority
        let path = resolve_java_path(Some("java21"), true, "system", &runtimes);
        assert_eq!(path, PathBuf::from("/usr/lib/jvm/java-21/bin/java"));

        // System fallback if instance says "system"
        let path = resolve_java_path(Some("system"), true, "", &runtimes);
        #[cfg(target_os = "windows")]
        assert_eq!(path, PathBuf::from("java.exe"));
        #[cfg(not(target_os = "windows"))]
        assert_eq!(path, PathBuf::from("java"));

        // Global default if instance not enabled
        runtimes.insert("default".to_string(), JavaRuntime {
            name: "Java Default".to_string(),
            path: "/usr/lib/jvm/java-default/bin/java".to_string(),
            version: 17,
            available: true,
        });
        let path = resolve_java_path(Some("java21"), false, "default", &runtimes);
        assert_eq!(path, PathBuf::from("/usr/lib/jvm/java-default/bin/java"));
    }
}
