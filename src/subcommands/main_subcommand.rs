use std::{fs::File, io::{Read, Write}};

use anyhow::{Context, Result};
use clap::ArgMatches;
use named_lock::NamedLock;
use tracing::info;
use zstd::Encoder;

use crate::{configuration::Configuration, credential_manager::CredentialManager, git_lfs::{error_init, CustomTransferAgent, Event, GitLfsParser, GitLfsProgressReporter}, synology_api::{ProgressReporter, SynologyFileStation}};

use super::Subcommand;

struct StdOutProgressReporter {
    git_lfs_progress_reporter: GitLfsProgressReporter,
    total_bytes: usize
}

impl ProgressReporter for StdOutProgressReporter {
    fn update(&mut self, bytes_so_far: usize) -> Result<()> {
        let progress = 0.9 * bytes_so_far as f64 / self.total_bytes as f64;
        self.git_lfs_progress_reporter.update(progress)
    }
}

#[derive(Debug)]
pub struct MainSubcommand {
    file_station: Option<SynologyFileStation>
}

impl CustomTransferAgent for MainSubcommand {
    #[tracing::instrument]
    async fn download(&mut self, _: &Event) -> Result<()> {
        Ok(())
    }

    #[tracing::instrument]
    async fn init(&mut self, _: &Event) -> Result<()> {
        let configuration = Configuration::load()?;
        let mut credential_manager = CredentialManager::new()?;

        let nas_url = configuration.nas_url.as_str();
        let mut file_station = SynologyFileStation::new(nas_url);

        let credential = credential_manager.get_credential(nas_url)?.context("Credential should not be null")?;
        match file_station.login(&credential).await {
            Ok(_) => Ok(()),
            Err(error) => {
                error_init(1, error.to_string().as_str())?;
                Err(error)
            }
        }?;

        self.file_station = Some(file_station);

        let path = configuration.path.as_str();
        match self.create_folder(path).await {
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

        let mut git_lfs_progress_reporter = GitLfsProgressReporter::new(
            event.size.clone().context("Size should not be null")?,
            event.oid.clone().context("oid should not be null")?);

        let path = event.path.clone().context("Path should not be null.")?;
        let (compressed, file) = self.compress_file(path.as_str(), event.size.context("Size should not be null")?, &mut git_lfs_progress_reporter)?;

        let target_file_name = if compressed {
            format!("{}.zst", event.oid.clone().context("oid should not be null")?)
        }
        else {
            event.oid.clone().context("oid should not be null")?
        };

        let path = format!(
            "{}/{}",
            configuration.path,
            target_file_name
        );

        // let mut progress_reporter = StdOutProgressReporter {
        //     git_lfs_progress_reporter
        // };

        // let file_station = self.file_station.clone().context("File Station should not be null")?;
        // file_station.upload(file, path.as_str(), false, false, None, None, None, Some(&mut progress_reporter)).await?;
        // Upload either the uncompressed blob or the original to the nas - 90%

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
    fn compress_file(&self, path: &str, size: usize, progress_reporter: &mut GitLfsProgressReporter) -> Result<(bool, File)> {
        const BYTES_TO_KB: usize = 1024;
        const KB_TO_MB: usize = 1024;
        const BYTES_TO_MB: usize = BYTES_TO_KB * KB_TO_MB;
        const CHUNK_SIZE: usize = 4 * BYTES_TO_MB;

        let chunk_count = (size as f64 / CHUNK_SIZE as f64).ceil() as u64;

        info!("Compressing file.  We have {} chunks.", chunk_count);

        let mut source = File::open(path)?;

        let target = tempfile::tempfile()?;
        let mut encoder = Encoder::new(&target, 0)?;

        let mut compressible = true;
        let mut buffer = [0; CHUNK_SIZE];
        for i in 0..chunk_count {
            let count = source.read(&mut buffer)?;
            let compressed_size = encoder.write(&buffer[..count])?;

            if i == 0 && compressed_size < count {
                info!("File is not compressible, aborting compression.");

                compressible = false;
                break // We are not compressible
            }

            let progress = 0.1 * (i + 1) as f64 / chunk_count as f64;
            progress_reporter.update(progress)?;
        }

        let progress = 1.0 / 10.0;
        progress_reporter.update(progress)?;

        if compressible {
            info!("Finished compressing.");

            encoder.finish()?;
            Ok((compressible, target))
        }
        else {
            info!("Compression is not possible");

            Ok((compressible, source))
        }
    }

    #[tracing::instrument]
    async fn create_folder(&self, path: &str) -> Result<()> {
        let configuration = Configuration::load()?;

        // This is a System wide, cross-process lock.
        let lock = NamedLock::create("git-lfs-synology::MianSubcommand::create_folder")?;
        let _guard = lock.lock()?;

        let file_station = self.file_station.clone().context("File Station should not be null.")?;

        let path_parts = configuration.path.split('/');
        let name = path_parts.last().context("Our path should have a name")?;
        // We remove one extra character so that we don't have a trailing '/'.
        let folder_path_string = configuration.path[..(configuration.path.len() - name.len() - 1)].to_string();
        let folder_path = folder_path_string.as_str();
        let _folders = file_station.create_folder(folder_path, name, true).await?;

        Ok(())
    }
}