use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use bridge::modal_action::ProgressTracker;
use schema;

/// Get the installer cache directory for a specific software and version
pub fn get_installer_cache_dir(server_dir: &Path, software: &str, version: &str) -> PathBuf {
    let launcher_parent = server_dir.parent().and_then(|p| p.parent()).unwrap_or_else(|| Path::new(""));
    let cache_dir = launcher_parent.join("server installers").join(software).join(version);
    let _ = std::fs::create_dir_all(&cache_dir);
    cache_dir
}

/// Get the installer path for a specific software type and version
pub fn get_installer_path(server_dir: &Path, software: &str, version: &str, filename: &str) -> PathBuf {
    let cache_dir = get_installer_cache_dir(server_dir, software, version);
    cache_dir.join(filename)
}

/// Install Fabric server with proper caching and validation
pub async fn install_fabric_server(
    client: &reqwest::Client,
    server_dir: &Path,
    version: &str,
    tracker: &ProgressTracker,
) -> Result<String, String> {
    log::info!("Starting Fabric {} installation", version);
    
    // Step 1: Delete old fabric-server-launch.jar if it exists (to avoid reusing broken installs)
    let target_jar = server_dir.join("fabric-server-launch.jar");
    let _ = tokio::fs::remove_file(&target_jar).await;
    log::info!("Deleted old fabric-server-launch.jar if it existed");
    
    // Step 2: Get vanilla minecraft server jar - check cache first
    tracker.set_title(Arc::from("Fetching Minecraft metadata..."));
    tracker.notify();
    
    let vanilla_jar_cache = get_installer_path(server_dir, "Vanilla", version, "server.jar");
    let vanilla_jar_path = server_dir.join("server.jar");
    
    // Try to use cached vanilla jar
    if vanilla_jar_cache.exists() {
        log::info!("Using cached vanilla server.jar");
        tracker.set_title(Arc::from("Using cached vanilla server JAR..."));
        tracker.notify();
        tokio::fs::copy(&vanilla_jar_cache, &vanilla_jar_path).await
            .map_err(|e| format!("Failed to copy cached vanilla JAR: {}", e))?;
    } else {
        // Download fresh vanilla jar and cache it
        log::info!("Downloading vanilla server.jar");
        tracker.set_title(Arc::from("Downloading vanilla server JAR..."));
        tracker.notify();
        
        let version_response = client.get("https://launchermeta.mojang.com/mc/game/version_manifest.json").send().await
            .map_err(|e| { log::error!("Failed to fetch manifest: {}", e); format!("Failed to fetch version manifest: {}", e) })?;
        
        let manifest_data = version_response.json::<serde_json::Value>().await
            .map_err(|e| { log::error!("Failed to parse manifest: {}", e); format!("Failed to parse manifest JSON: {}", e) })?;
        
        let versions = manifest_data["versions"].as_array()
            .ok_or_else(|| { log::error!("No versions in manifest"); "No versions in manifest".to_string() })?;
        
        let version_entry = versions.iter()
            .find(|v| v["id"].as_str().map(|id| id == version).unwrap_or(false))
            .ok_or_else(|| { log::error!("Version {} not found", version); format!("Minecraft version {} not found", version) })?;
        
        let url = version_entry["url"].as_str()
            .ok_or_else(|| { log::error!("Version URL not found"); "Version URL not found in manifest".to_string() })?;
        
        let version_response = client.get(url).send().await
            .map_err(|e| { log::error!("Failed to fetch version.json: {}", e); format!("Failed to fetch version JSON: {}", e) })?;
        
        let version_json = version_response.json::<serde_json::Value>().await
            .map_err(|e| { log::error!("Failed to parse version.json: {}", e); format!("Failed to parse version JSON: {}", e) })?;
        
        let download_url = version_json["downloads"]["server"]["url"].as_str()
            .ok_or_else(|| { log::error!("Server JAR URL not found"); "Server JAR URL not found".to_string() })?;
        
        tracker.set_title(Arc::from(format!("Downloading Minecraft {} server JAR...", version)));
        tracker.notify();
        
        let jar_response = client.get(download_url).send().await
            .map_err(|e| { log::error!("Failed to download JAR: {}", e); format!("Failed to download Minecraft JAR: {}", e) })?;
        
        if let Some(content_length) = jar_response.content_length() {
            tracker.set_total(content_length as usize);
            tracker.notify();
        }
        
        let jar_bytes = jar_response.bytes().await
            .map_err(|e| { log::error!("Failed to read JAR bytes: {}", e); format!("Failed to read JAR bytes: {}", e) })?;
        
        if jar_bytes.len() > 0 {
            tracker.set_count(jar_bytes.len());
            tracker.notify();
        }
        
        log::info!("Downloaded vanilla JAR: {} bytes", jar_bytes.len());
        
        // Write to both server dir and cache
        tokio::fs::write(&vanilla_jar_path, &jar_bytes).await
            .map_err(|e| { log::error!("Failed to write vanilla JAR to server dir: {}", e); format!("Failed to write vanilla JAR: {}", e) })?;
        
        // Cache the vanilla jar
        if let Err(e) = tokio::fs::write(&vanilla_jar_cache, &jar_bytes).await {
            log::warn!("Failed to cache vanilla JAR: {}", e);
        }
    }
    
    // Step 3: Get latest Fabric installer version
    tracker.set_title(Arc::from("Fetching Fabric installer versions..."));
    tracker.notify();
    
    let installer_meta = client.get("https://maven.fabricmc.net/net/fabricmc/fabric-installer/maven-metadata.xml").send().await
        .map_err(|e| { log::error!("Failed to fetch installer versions: {}", e); format!("Failed to fetch Fabric installer versions: {}", e) })?;
    
    let installer_meta_xml = installer_meta.text().await
        .map_err(|e| { log::error!("Failed to read installer metadata: {}", e); format!("Failed to read installer metadata: {}", e) })?;
    
    let latest_installer_version = extract_latest_version_from_maven_metadata(&installer_meta_xml)
        .unwrap_or_else(|| "1.1.2".to_string()); // Fallback to known good version
    
    log::info!("Using Fabric installer version: {}", latest_installer_version);
    
    // Step 4: Cache installer
    let installer_filename = format!("fabric-installer-{}.jar", latest_installer_version);
    let installer_path = get_installer_path(server_dir, "Fabric", version, &installer_filename);
    
    if !installer_path.exists() {
        log::info!("Downloading Fabric installer {}", latest_installer_version);
        tracker.set_title(Arc::from(format!("Downloading Fabric installer {}...", latest_installer_version)));
        tracker.notify();
        
        let installer_url = format!(
            "https://maven.fabricmc.net/net/fabricmc/fabric-installer/{}/fabric-installer-{}.jar",
            latest_installer_version, latest_installer_version
        );
        
        let installer_response = client.get(&installer_url).send().await
            .map_err(|e| { log::error!("Failed to download installer: {}", e); format!("Failed to download Fabric installer: {}", e) })?;
        
        if let Some(content_length) = installer_response.content_length() {
            tracker.set_total(content_length as usize);
            tracker.notify();
        }
        
        let installer_bytes = installer_response.bytes().await
            .map_err(|e| { log::error!("Failed to read installer bytes: {}", e); format!("Failed to read installer bytes: {}", e) })?;
        
        if installer_bytes.len() > 0 {
            tracker.set_count(installer_bytes.len());
            tracker.notify();
        }
        
        tokio::fs::write(&installer_path, installer_bytes).await
            .map_err(|e| { log::error!("Failed to cache installer: {}", e); format!("Failed to cache Fabric installer: {}", e) })?;
    } else {
        log::info!("Using cached Fabric installer");
        tracker.set_title(Arc::from(format!("Using cached Fabric installer {}...", latest_installer_version)));
        tracker.notify();
    }
    
    // Step 5: Run installer and validate
    tracker.set_title(Arc::from("Running Fabric installer..."));
    tracker.notify();
    
    log::info!("Running Fabric installer for version {}", version);
    let install_start_time = SystemTime::now();
    
    let output = std::process::Command::new("java")
        .arg("-jar")
        .arg(&installer_path)
        .arg("server")
        .arg("-dir")
        .arg(&server_dir)
        .arg("-mcversion")
        .arg(&version)
        .output();
    
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            log::info!("Fabric installer stdout: {}", stdout);
            log::info!("Fabric installer stderr: {}", stderr);
            log::info!("Fabric installer exit code: {}", output.status);
            
            // Validate installation - check that jar was created AFTER install started
            if let Err(e) = validate_installation(&target_jar, install_start_time) {
                log::error!("Installation validation failed: {}", e);
                return Err(format!("Installation validation failed: {}. Installer output: {}", e, stderr));
            }
            
            log::info!("Fabric installation successful! Jar exists at: {:?}", target_jar);
            tracker.set_title(Arc::from(format!("✓ Fabric {} server installed successfully", version)));
            Ok(target_jar.file_name().unwrap_or_default().to_string_lossy().to_string())
        }
        Err(e) => {
            log::error!("Failed to run Fabric installer: {}", e);
            Err(format!("Failed to run Fabric installer: {}", e))
        }
    }
}

/// Extract latest version from Maven metadata XML
fn extract_latest_version_from_maven_metadata(xml: &str) -> Option<String> {
    // Simple regex-based parsing for latest version tag
    if let Some(start) = xml.find("<latest>") {
        if let Some(end) = xml[start + 8..].find("</latest>") {
            return Some(xml[start + 8..start + 8 + end].to_string());
        }
    }
    None
}

/// Validate that installation succeeded by checking file existence and modification time
pub fn validate_installation(
    target_jar: &Path,
    before_time: SystemTime,
) -> Result<(), String> {
    if !target_jar.exists() {
        return Err(format!("Installer did not create expected jar: {:?}", target_jar));
    }
    
    // Check that jar was modified after installation started
    if let Ok(metadata) = std::fs::metadata(target_jar) {
        if let Ok(modified) = metadata.modified() {
            if modified < before_time {
                return Err(format!(
                    "Jar file was not created by this installation (older than installation start)"
                ));
            }
        }
    }
    
    Ok(())
}

/// Finds the most appropriate JAR file to start the server
pub fn find_server_jar(server_dir: &Path) -> String {
    if let Ok(entries) = std::fs::read_dir(server_dir) {
        let mut jar_files = Vec::new();
        for entry in entries.flatten() {
            if entry.path().extension().map_or(false, |ext| ext == "jar") {
                jar_files.push(entry.file_name());
            }
        }
        
        // Priority 1: Fabric launch JAR
        if jar_files.iter().any(|f| f == "fabric-server-launch.jar") {
            return "fabric-server-launch.jar".to_string();
        }
        
        // Priority 2: Forge/NeoForge server/universal JARs (not installers)
        for jar in &jar_files {
            let name = jar.to_string_lossy().to_lowercase();
            if (name.contains("forge") || name.contains("neoforge")) 
                && (name.contains("server") || name.contains("universal")) 
                && !name.contains("installer") 
            {
                return jar.to_string_lossy().to_string();
            }
        }

        // Priority 3: First available JAR that isn't an installer
        if let Some(jar) = jar_files.iter().find(|f| !f.to_string_lossy().to_lowercase().contains("installer")) {
            return jar.to_string_lossy().to_string();
        }
    }
    "server.jar".to_string()
}

/// Generate a start.sh script for a server with memory and JVM settings
pub fn generate_start_script(
    server_dir: &Path,
    java_executable: &str,
    jar_filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Try to read server config for memory and JVM settings
    let config_path = server_dir.join(".minecraft").join("server_config.json");
    log::debug!("Reading config from: {}", config_path.display());
    let (memory_config, jvm_flags) = if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            log::debug!("Config content: {}", content);
            if let Ok(config) = serde_json::from_str::<schema::server_config::ServerConfiguration>(&content) {
                let mem = config.java
                    .as_ref()
                    .and_then(|java| java.memory);
                let flags = config.java
                    .as_ref()
                    .and_then(|java| {
                        if java.jvm_flags.is_empty() {
                            None
                        } else {
                            Some(java.jvm_flags.clone())
                        }
                    });
                log::debug!("Parsed memory: {:?}, flags: {:?}", mem, flags);
                (mem, flags)
            } else {
                log::warn!("Failed to parse config JSON");
                (None, None)
            }
        } else {
            log::warn!("Could not read config file");
            (None, None)
        }
    } else {
        log::debug!("Config file does not exist: {}", config_path.display());
        (None, None)
    };
    
    // Get memory settings with defaults
    let (min_mem, max_mem) = if let Some(mem) = memory_config {
        (mem.min, mem.max)
    } else {
        (512, 1024) // defaults
    };
    
    // Build the script content with proper memory settings and JVM flags
    let jvm_args = if let Some(flags) = jvm_flags {
        format!("-Xms{}M -Xmx{}M {}", min_mem, max_mem, flags)
    } else {
        format!("-Xms{}M -Xmx{}M", min_mem, max_mem)
    };
    
    log::debug!("Generated JVM args: {}", jvm_args);
    
    let script_content = format!(
        "#!/bin/bash\ncd \"$( cd \"$( dirname \"${{BASH_SOURCE[0]}}\" )\" && pwd )\"\nexec \"{java}\" {args} -jar \"{jar}\" nogui\n",
        java = java_executable,
        args = jvm_args,
        jar = jar_filename
    );
    
    log::info!("Writing start.sh with content");
    let script_path = server_dir.join("start.sh");
    std::fs::write(&script_path, script_content)?;
    log::info!("Wrote start.sh to: {}", script_path.display());
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&script_path, perms)?;
    }
    
    Ok(())
}
