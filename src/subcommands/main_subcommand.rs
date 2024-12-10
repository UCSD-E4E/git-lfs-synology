use std::{fs::{exists, File}, path::{Path, PathBuf}};

use anyhow::{Context, Result};
use clap::ArgMatches;
use named_lock::NamedLock;
use tokio::fs::remove_file;
use tracing::info;

use crate::{configuration::Configuration, credential_manager::CredentialManager, git_lfs::{error_init, CustomTransferAgent, Event, GitLfsParser, GitLfsProgressReporter}, synology_api::{ProgressReporter, SynologyFileStation}, users_dirs::get_cache_dir};

use super::Subcommand;

#[derive(Debug)]
struct StdOutProgressReporter {
    git_lfs_progress_reporter: GitLfsProgressReporter
}

impl ProgressReporter for StdOutProgressReporter {
    fn update(&mut self, bytes_since_last: usize) -> Result<()> {
        self.git_lfs_progress_reporter.update(bytes_since_last)
    }
}

#[derive(Debug)]
pub struct MainSubcommand {
    file_station: Option<SynologyFileStation>
}

impl CustomTransferAgent for MainSubcommand {
    #[tracing::instrument]
    async fn download(&mut self, event: &Event) -> Result<PathBuf> {
        let configuration = Configuration::load()?;
        let oid = event.oid.clone().context("OID should not be null")?;

        let git_lfs_progress_reporter = GitLfsProgressReporter::new(
            event.size.context("Size should not be null")?,
            event.oid.clone().context("oid should not be null")?);

        let mut source_file_path = format!(
            "{}/{}",
            configuration.path,
            oid
        );

        let compressed_file_path = format!(
            "{}.zstd",
            source_file_path
        );

        let mut source_file_compressed = false;
        if self.exists_on_remote(&compressed_file_path).await? {
            source_file_path = compressed_file_path;
            source_file_compressed = true;
        }

        info!("Source path is \"{}\".", source_file_path);

        let mut target_directory_path = PathBuf::new();
        target_directory_path.push(".");
        target_directory_path.push(".git");
        target_directory_path.push("lfs");
        target_directory_path.push("objects");
        target_directory_path.push(&oid[..2]);
        target_directory_path.push(&oid[2..4]);
        
        info!("Target path is \"{}\".", target_directory_path.as_os_str().to_string_lossy());

        let progress_reporter = StdOutProgressReporter {
            git_lfs_progress_reporter
        };

        let file_station = self.file_station.clone().context("File Station should not be null")?;
        let mut target_file_path = file_station.download(source_file_path.as_str(), target_directory_path.as_path(), Some(progress_reporter)).await?;

        if source_file_compressed {
            target_file_path = self.uncompress_file(&target_file_path)?;
        }

        info!("Download finished");
        Ok(target_file_path)
    }

    #[tracing::instrument]
    async fn init(&mut self, _: &Event) -> Result<()> {
        let configuration = Configuration::load()?;
        let mut credential_manager = CredentialManager::new()?;

        let nas_url = configuration.nas_url.as_str();
        let mut file_station = SynologyFileStation::new(nas_url);

        let credential = credential_manager.get_credential(nas_url)?.context("Credential should not be null")?;
        match file_station.login(&credential, false, None).await {
            Ok(_) => Ok(()),
            Err(error) => {
                error_init(1, error.to_string().as_str())?;
                Err(error)
            }
        }?;

        self.file_station = Some(file_station);
        
        match self.create_target_folder().await {
            Ok(_) => Ok(()),
            Err(error) => {
                error_init(1, error.to_string().as_str())?;
                Err(error)
            }
        }
    }

    #[tracing::instrument]
    async fn terminate(&mut self) -> Result<()> {
        // No cleanup to do.

        Ok(())
    }

    #[tracing::instrument]
    async fn upload(&mut self, event: &Event) -> Result<()> {
        let configuration = Configuration::load()?;

        let git_lfs_progress_reporter = GitLfsProgressReporter::new(
            event.size.context("Size should not be null")?,
            event.oid.clone().context("oid should not be null")?);

        let event_source_path = event.path.clone().context("Path should not be null.")?;
        info!("Preparing to upload file at \"{}\".", event_source_path);
        info!("Pushing to server path: \"{}\".", configuration.path);

        let progress_reporter = StdOutProgressReporter {
            git_lfs_progress_reporter
        };

        info!("Attempting to compress the source file.");
        let compressed_source_path = self.compress_file(&event_source_path).await?;

        let source_path = Path::new(&compressed_source_path);
        let target_path = format!(
            "{}/{}",
            configuration.path,
            event.oid.clone().context("OID should not be none.")?
        );

        if self.exists_on_remote_compressed_or_uncompressed(target_path.as_str()).await? {
            info!("Object already exists on server.");

            return Ok(())
        }

        let file_station = self.file_station.clone().context("File Station should not be null")?;
        file_station.upload(source_path, event.size.context("Size should not be null")?, configuration.path.as_str(), false, false, None, None, None, Some(progress_reporter)).await?;

        // Remove the path if the compressed source path is not the same as the source path provided by git lfs.
        if event_source_path != compressed_source_path {
            let path = Path::new(&compressed_source_path);
            remove_file(path).await?;
        }

        info!("Upload finished.");
        Ok(())
    }
}

impl Subcommand for MainSubcommand {
    #[tracing::instrument]
    async fn execute(&mut self, arg_matches: &ArgMatches) -> Result<()> {
        let mut parser = GitLfsParser::<MainSubcommand>::new(self);
        parser.listen().await?;

        Ok(())
    }
}

impl MainSubcommand {
    #[tracing::instrument]
    pub fn new() -> MainSubcommand {
        MainSubcommand {
            file_station: None
        }
    }

    #[tracing::instrument]
    async fn compress_file(&self, path: &str) -> Result<String> {
        let source_file = Path::new(path);
        let mut compress_file = get_cache_dir()?;
        compress_file.push(
            format!("{}.zstd", source_file.file_name().context("File name should not be null")?.to_string_lossy())
        );

        if exists(&compress_file)? {
            info!("File already exists, deleting it.");

            remove_file(&compress_file).await?;
        }

        let source_file = File::open(source_file)?;
        let target_file = File::create(&compress_file)?;

        zstd::stream::copy_encode(&source_file, &target_file, 0)?;

        if target_file.metadata()?.len() < source_file.metadata()?.len() {
            info!("Compressed file is not smaller.");

            // Remove the file, it is not necessary to maintain this.
            remove_file(compress_file).await?;

            Ok(path.to_string())
        }
        else {
            info!("Compressed file is smaller.");

            Ok(compress_file.to_str().context("Compress path should not be null.")?.to_string())
        }
    }

    #[tracing::instrument]
    async fn create_target_folder(&self) -> Result<()> {
        let configuration = Configuration::load()?;

        if self.exists_on_remote(&configuration.path).await? {
            return Ok(()); // Exit early, handle trying to create a folder over a share.
        }

        // This is a System wide, cross-process lock.
        let lock = NamedLock::create("git-lfs-synology::MainSubcommand::create_target_folder")?;
        let _guard = lock.lock()?;

        let file_station = self.file_station.clone().context("File Station should not be null.")?;

        let name = self.get_name(&configuration.path)?;
        let folder_path = self.get_parent_path(&configuration.path)?.context("Path should not be root.")?;
        let _folders = file_station.create_folder(folder_path.as_str(), name.as_str(), true).await?;

        Ok(())
    }

    #[tracing::instrument]
    async fn exists_on_remote(&self, path: &str) -> Result<bool> {
        if self.is_path_root(path) {
            info!("Path is root.");

            return Ok(true); // The root should always exist.  Don't need to ask the server to confirm.
        }

        let name = self.get_name(path)?;
        let parent = self.get_parent_path(path)?.context("Path should not be root since we checked earlier.")?;

        let file_station = self.file_station.clone().context("File Station should not be null")?;

        if self.is_path_root(&parent) {
            info!("Parent is root, let's get shares.");

            let shares = file_station.list_share(
                None, None, None, None, None,
                false, false, false, false, false, false, false).await?;

            return Ok(shares.shares.iter().any(|share| share.name == name));
        }
        else {
            info!("Parent is not root, let's get files.");

            let files = file_station.list(
                &parent, None, None, None, None, None, None, None,
                false, false, false, false, false, false, false).await?;

            return Ok(files.files.iter().any(|file| file.name == name));
        }
    }

    #[tracing::instrument]
    async fn exists_on_remote_compressed_or_uncompressed(&self, path: &str) -> Result<bool> {
        let compressed_path = format!("{}.zstd", path);

        Ok(self.exists_on_remote(path).await? || self.exists_on_remote(&compressed_path).await?)
    }

    #[tracing::instrument]
    fn get_parent_path(&self, path: &str) -> Result<Option<String>> {
        if self.is_path_root(path) {
            return Ok(None)
        }

        let path_parts = path.split('/');
        let name = path_parts.last().context("Our path should have a name since it's not the root.")?;
        // We remove one extra character so that we don't have a trailing '/'.
        Ok(Some(path[..(path.len() - name.len() - 1)].to_string()))
    }

    #[tracing::instrument]
    fn get_name(&self, path: &str) -> Result<String> {
        if self.is_path_root(path) {
            return Ok("".to_string()); // We are the root.  We don't have a name.
        }

        let path_parts = path.split('/');
        let name = path_parts.last().context("Our path should have a name since it's not the root.")?;

        Ok(name.to_string())
    }

    #[tracing::instrument]
    fn is_path_root(&self, path: &str) -> bool {
        path == "/" || path.is_empty()
    }

    fn uncompress_file(&self, source_path: &PathBuf) -> Result<PathBuf> {
        if let Some(extension) = source_path.extension() {
            if extension == ".zstd" {
                let source_path_string = source_path.to_string_lossy();
                let split_pos = source_path_string.char_indices().nth_back(5).context("Should have more than 5 characters.")?.0;
                let mut target_path = PathBuf::new();
                target_path.push(&source_path_string[..split_pos]);

                let source_file = File::open(source_path)?;
                let target_file = File::create(&target_path)?;

                zstd::stream::copy_decode(source_file, target_file)?;

                return Ok(target_path);
            }
        }

        Ok(source_path.clone())
    }
}