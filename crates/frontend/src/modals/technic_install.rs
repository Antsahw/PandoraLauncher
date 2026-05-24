use std::sync::Arc;

use bridge::{install::{ContentDownload, ContentInstall, ContentInstallFile, InstallTarget}, instance::InstanceID};
use gpui::*;
use schema::content::ContentSource;

use crate::{entity::DataEntities, root};

pub fn open(
    modpack_name: Arc<str>,
    install_for: Option<InstanceID>,
    data: &DataEntities,
    window: &mut Window,
    cx: &mut App,
) {
    if let Some(instance_id) = install_for {
        let content_install = ContentInstall {
            target: InstallTarget::Instance(instance_id),
            loader_hint: schema::loader::Loader::Vanilla,
            version_hint: None,
            files: [
                ContentInstallFile {
                    replace_old: None,
                    path: bridge::install::ContentInstallPath::Automatic,
                    download: ContentDownload::Technic {
                        modpack_name: modpack_name.clone(),
                        version: "latest".into(),
                    },
                    content_source: ContentSource::TechnicModpack {
                        modpack_name: modpack_name.clone()
                    },
                }
            ].into(),
        };

        root::start_install(content_install, &data.backend_handle, window, cx);
    }
}
