
use anyhow::{Result as Res, Context, anyhow};
use crate::directories::ProjectDirs;
use std::{path::{Path, PathBuf}, fs, io::{self, ErrorKind::NotFound}};


#[derive(Debug, Clone)]
pub struct LocalStorage {
    storage_dir: PathBuf,
}

impl LocalStorage {

    pub fn path(&self) -> &Path { self.storage_dir.as_ref() }

    pub fn from_path(storage_dir: PathBuf) -> Res<Self> {

        fs::create_dir_all(&storage_dir).with_context(|| format!("couldn't acquire storage dir {:?}", storage_dir.display()))?;

        Ok(Self {storage_dir})
    }

    pub fn new(qualifier: &str, organization: &str, application: &str) -> Res<Self> {

        let project_dirs = ProjectDirs::from(qualifier, organization, application).context("failed to acquire ProjectDirs")?;
        let storage_dir: PathBuf = project_dirs.config_local_dir().into();

        Self::from_path(storage_dir)
    }

    pub fn set(&self, key: &str, value: &str) -> Res<()> {
        let path = self.storage_dir.join(key);
        fs::write(&path, value).with_context(|| format!("failed writing to {:?}", path.display()))
    }

    pub fn get(&self, key: &str) -> Res<Option<String>> {
        let path = self.storage_dir.join(key);
        match fs::read_to_string(&path) {
            Ok(value) => Ok(Some(value)),
            Err(err) => {
                if err.kind() == NotFound { Ok(None) }
                else { Err(err).with_context(|| format!("failed reading from {:?}", path.display())) }
            }
        }
    }

    pub fn remove(&self, key: &str) -> Res<()> {
        let path = self.storage_dir.join(key);
        fs::remove_file(&path).with_context(|| format!("failed to remove {:?}", path.display()))
    }

    pub fn clear_all(&self) -> Res<()> {

        let read_dir = fs::read_dir(&self.storage_dir).with_context(|| format!("failed reading {:?}", self.storage_dir.display()))?;

        let mut errors = Vec::new();

        for res in read_dir { match res {

            Err(err) => errors.push(format!("{:?}", err)),

            Ok(entry) => {
                let path = entry.path();

                if let Err(err) = |entry: fs::DirEntry| -> io::Result<()> {
                    if entry.metadata()?.is_file() {
                        fs::remove_file(&path)?;
                    }
                    Ok(())
                } (entry) {
                    errors.push(format!("{:?}", anyhow!("{:?}: {:?}", path.display(), err)));
                }
            }

        }}

        if errors.is_empty() { Ok(()) }
        else { Err(anyhow!(errors.join("\n"))) }
    }
}