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

## Installing

### macOS
```bash
brew install git-lfs
```

### Published Release (Preferred)
Download the most recent [release](https://github.com/UCSD-E4E/git-lfs-synology/releases) and ensure the binary is on your path.

### Cargo
Install all the development dependencies and then execute the following. Note that this will not properly set the version and should only be used for testing.
```bash
cargo install --git https://github.com/UCSD-E4E/git-lfs-synology.git git-lfs-synology
```

## Uninstalling

### Cargo
```bash
cargo uninstall git-lfs-synology
```

## Configuring Git LFS
It is necessary for you to provide `git-lfs-synology` your password and a method to generate TOTP codes. It stores your username and password in your operating system's keyring and uses your password to encrypt your TOTP command if one is provided. It stores the encrypted TOTP command within a sqlite database in your system's configuration directory. To store your username and password, perform these steps.

### Logging In

#### No TOTP command
```bash
git-lfs-synology login --url https://e4e-nas.ucsd.edu:6021 --user <username> # Ensure you update your username
```

#### TOTP command
```bash
git-lfs-synology login --url https://e4e-nas.ucsd.edu:6021 --user <username> --totp-command "<totp_command>" # Ensure you update your username and totp command
```

### Globally
Run these steps to update your git config globally.  You may not want to perform these if you use other Git LFS implementations.

#### Bash
```bash
git config --global lfs.standalonetransferagent git-lfs-synology
git config --global lfs.customtransfer.git-lfs-synology.path `which git-lfs-synology`
```

### Locally
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