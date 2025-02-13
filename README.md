# git-lfs-synology
This is a git lfs custom transfer agent for Synology File Station.  See [this](https://github.com/git-lfs/git-lfs/blob/main/docs/custom-transfers.md) page for documentation on the expected communication protocol. An additional resource for understanding a git lfs custom agent is `lfs-dal` on [GitHub](https://github.com/regen100/lfs-dal).

## Development Dependencies

### macOS
```bash
brew install cmake
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Ubuntu
```bash
sudo apt-get install -y libdbus-1-dev pkg-config curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Dependencies

### macOS
```bash
brew install git-lfs
```

### Ubuntu
```bash
sudo apt-get install git-lfs
```

## Installing

### Published Release (Preferred)
Download the most recent [release](https://github.com/UCSD-E4E/git-lfs-synology/releases) and ensure the binary is on your path. We have provided install scripts for simplicity.

#### Windows
These install scripts install `git-lfs-synology` globally. This will impact all git repos on your system. Please install manually if this is something that you need to avoid.

Please install this using PowerShell as your user. Do NOT execute this from an admin shell.

```powershell
Set-ExecutionPolicy Bypass -Scope Process -Force; Invoke-Expression (Invoke-WebRequest https://raw.githubusercontent.com/UCSD-E4E/git-lfs-synology/refs/heads/main/scripts/install.ps1?bust=$((Get-Date).Ticks)).Content; Invoke-InstallScript
```

### Cargo
Install all the development dependencies and then execute the following.
```bash
cargo install --git https://github.com/UCSD-E4E/git-lfs-synology.git git-lfs-synology --branch main
```

## Uninstalling

### Cargo
```bash
cargo uninstall git-lfs-synology
```

## Configuring Git LFS
It is necessary for you to provide `git-lfs-synology` your username and password. It stores your username and password in your operating system's keyring. To store your username and password, perform these steps.

### Logging In

```bash
git-lfs-synology login --url https://e4e-nas.ucsd.edu:6021 --user <username> # Ensure you update your username
```

### Configuring the Custom Transfer Agent Globally
Run these steps to update your git config globally.  You may not want to perform these if you use other Git LFS implementations.

#### Bash
```bash
git config --global lfs.standalonetransferagent git-lfs-synology
git config --global lfs.customtransfer.git-lfs-synology.path `which git-lfs-synology`
```

### Configuring the Custom Transfer Agent Locally
Run these steps locally within the repository you want to setup Git LFS Synology in if you don't want to update your global git settings.

#### Bash
```bash
git config lfs.standalonetransferagent git-lfs-synology
git config lfs.customtransfer.git-lfs-synology.path `which git-lfs-synology`
```

## Setup a Repository
```bash
cd repo
git lfs install --local
git config -f .lfsconfig lfs.url filestation-secure://e4e-nas.ucsd.edu:6021/<share-name>/git-lfs/<repo-name> # Ensure that you update <share-name> and <repo-name>.
# Ensure that you have followed the steps in "the Configuring Git LFS" section.
git lfs track *.EXT # Replace *.EXT with the objects you want git lfs to track.
```

## Downloading a Repository
```bash
git clone https://example.com/git/repo
cd repo
git lfs install --local
# Ensure that you have followed the steps in "the Configuring Git LFS" section.
git lfs pull
```