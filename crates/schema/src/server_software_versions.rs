use serde::{Deserialize, Serialize};

/// Available versions for a specific server software
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSoftwareVersions {
    pub software: String,
    pub versions: Vec<String>,
}
