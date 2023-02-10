use crate::config::Config;
use anyhow::Result;
use flate2::read::GzDecoder;
use self_update::Download;
use std::fs::File;
use tar::Archive;

pub struct RemoteTemplate<'a> {
    config: &'a Config,
    url: url::Url,
}

impl<'a> RemoteTemplate<'a> {
    pub fn new(config: &'a Config) -> RemoteTemplate {
        RemoteTemplate {
            config,
            url: config.template().registry,
        }
    }

    pub fn download(&self) -> Result<()> {
        let tmp_dir = tempfile::Builder::new().prefix("ion-templates").tempdir()?;
        let fname = tmp_dir.path().join("ion-templates.tar.gz");
        let dest = File::create(&fname)?;
        Download::from_url(self.url.as_str())
            .show_progress(true)
            .download_to(dest)?;

        log::debug!("downloaded to: '{:?}'", fname.display());
        let tar_gz = File::open(&fname)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        let resources_dir = self.config.resources();
        if !resources_dir.exists() {
            std::fs::create_dir_all(&resources_dir)?;
        }
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
