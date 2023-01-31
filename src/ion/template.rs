use crate::utils::resources_dir;
use anyhow::Result;
use flate2::read::GzDecoder;
use self_update::Download;
use std::fs::File;
use tar::Archive;

pub struct RemoteTemplate {
    url: String,
}

impl Default for RemoteTemplate {
    fn default() -> Self {
        Self {
            url: String::from(
                "https://github.com/Roger-luo/\
            ion-templates/releases/latest/download/ion-templates.tar.gz",
            ),
        }
    }
}

impl RemoteTemplate {
    pub fn new(url: String) -> RemoteTemplate {
        RemoteTemplate { url }
    }

    pub fn download(&self) -> Result<()> {
        let tmp_dir = tempfile::Builder::new().prefix("ion-templates").tempdir()?;
        let fname = tmp_dir.path().join("ion-templates.tar.gz");
        let dest = File::create(&fname)?;
        Download::from_url(&self.url)
            .show_progress(true)
            .download_to(dest)?;

        log::debug!("downloaded to: '{:?}'", fname.display());
        let tar_gz = File::open(&fname)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        let resources_dir = resources_dir()?;
        archive.entries()?.filter_map(|e| e.ok()).for_each(|mut e| {
            let path = e.path().unwrap();
            let path = path.strip_prefix("dist").unwrap();
            let path = resources_dir.join(path);
            log::debug!("unpacking: '{:?}'", path);
            e.unpack(&path).unwrap();
        });
        Ok(())
    }
}
