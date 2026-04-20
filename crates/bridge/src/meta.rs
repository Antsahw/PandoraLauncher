use std::sync::Arc;

use schema::{curseforge::{CurseforgeGetModFilesRequest, CurseforgeGetModFilesResult, CurseforgeSearchRequest, CurseforgeSearchResult}, fabric_loader_manifest::FabricLoaderManifest, forge::{ForgeMavenManifest, NeoforgeMavenManifest}, modrinth::{ModrinthProjectRequest, ModrinthProjectResult, ModrinthProjectVersionsRequest, ModrinthProjectVersionsResult, ModrinthSearchRequest, ModrinthSearchResult}, server_software_versions::ServerSoftwareVersions, version_manifest::MinecraftVersionManifest};

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum MetadataRequest {
    MinecraftVersionManifest,
    FabricLoaderManifest,
    ForgeMavenManifest,
    NeoforgeMavenManifest,
    ServerSoftwareVersions(String), // software name: "Paper", "Fabric", etc.
    ModrinthSearch(ModrinthSearchRequest),
    ModrinthProjectVersions(ModrinthProjectVersionsRequest),
    ModrinthProject(ModrinthProjectRequest),
    CurseforgeSearch(CurseforgeSearchRequest),
    CurseforgeGetModFiles(CurseforgeGetModFilesRequest),
}

#[derive(Debug)]
pub enum MetadataResult {
    MinecraftVersionManifest(Arc<MinecraftVersionManifest>),
    FabricLoaderManifest(Arc<FabricLoaderManifest>),
    ForgeMavenManifest(Arc<ForgeMavenManifest>),
    NeoforgeMavenManifest(Arc<NeoforgeMavenManifest>),
    ServerSoftwareVersions(Arc<ServerSoftwareVersions>),
    ModrinthSearchResult(Arc<ModrinthSearchResult>),
    ModrinthProjectVersionsResult(Arc<ModrinthProjectVersionsResult>),
    ModrinthProjectResult(Arc<ModrinthProjectResult>),
    CurseforgeSearchResult(Arc<CurseforgeSearchResult>),
    CurseforgeGetModFilesResult(Arc<CurseforgeGetModFilesResult>),
}
