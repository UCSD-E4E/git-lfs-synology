use anyhow::{Context, Result};
use clap::ArgMatches;
use named_lock::NamedLock;
use tracing::info;

use crate::{configuration::Configuration, credential_manager::CredentialManager, git_lfs::{error_init, CustomTransferAgent, Event, GitLfsParser, GitLfsProgressReporter}, synology_api::{ProgressReporter, SynologyFileStation}};

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

        let git_lfs_progress_reporter = GitLfsProgressReporter::new(
            event.size.clone().context("Size should not be null")?,
            event.oid.clone().context("oid should not be null")?);

        let source_path = event.path.clone().context("Path should not be null.")?;
        info!("Preparing to upload file at \"{}\".", source_path);
        info!("Pushing to server path: \"{}\".", configuration.path);

        let progress_reporter = StdOutProgressReporter {
            git_lfs_progress_reporter
        };

        let file_station = self.file_station.clone().context("File Station should not be null")?;
        file_station.upload(source_path.as_str(), event.size.clone().context("Size should not be null")?, configuration.path.as_str(), false, false, None, None, None, Some(progress_reporter)).await?;

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