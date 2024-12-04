function Invoke-InstallScript {
    param (
        [Parameter(Mandatory=$true)]
        [string]$URL,
        [Parameter(Mandatory=$true)]
        [string]$User
    )

    # Get all releases
    $releases = ConvertFrom-Json (Invoke-WebRequest "https://api.github.com/repos/ucsd-e4e/git-lfs-synology/releases").Content

    # Get the latest release
    $maxDate = ($releases.published_at | Measure-Object -Maximum).Maximum
    $latestRelease = $releases | Where-Object { $_.published_at -eq $maxDate }

    # Determine OS and Architecture
    $osPlatform = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
    $architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture

    # Adjust the platform and architecture for the API call
    $platform = switch -Wildcard ($osPlatform) {
        "*Windows*" { "win" }
        "*Linux*"   { "linux" }
        "*Darwin*"  { "osx" } # MacOS is identified as Darwin
        Default     { "unknown" }
    }
    $arch = switch ($architecture) {
        "X64"  { "x86_64" }
        "Arm64" { "aarch64" }
        Default { "unknown" }
    }

    # Construct the asset name.
    $assetName = "git-lfs-synology.$platform-$arch.zip"

    # Get the Asset
    $asset = $latestRelease.assets | Where-Object { $_.name -eq $assetName }

    if ($null -eq $asset) {
        Write-Error "$assetName is not published.  Please file an issue."
    }

    $homeDirectory = Get-Item ~
    $gitLfsSynologyDirectory = ".git-lfs-synology"
    $targetPath = Join-Path $homeDirectory $gitLfsSynologyDirectory

    if (Test-Path $targetPath) {
        Remove-Item $targetPath -Force -Recurse | Out-Null
    }
    New-Item -Path $homeDirectory -Name $gitLfsSynologyDirectory -ItemType Directory | Out-Null

    Push-Location $targetPath

    # Get the executable
    Invoke-WebRequest $asset.browser_download_url -OutFile $asset.name | Out-Null
    Expand-Archive -Path $asset.name -DestinationPath $targetPath

    Pop-Location

    # Update the Path Environment Variable
    if (-not ($targetPath -in $env:PATH)) {
        if ($IsWindows) {
            $seperator = ";"
        }
        else {
            $seperator = ":"
        }

        $env:PATH = "$($targetPath)$($seperator)$($env:PATH)"

        if ($IsWindows) {
            # This only works on Windows.
            [Environment]::SetEnvironmentVariable("Path", $env:Path, [System.EnvironmentVariableTarget]::Machine)
        }
        else {
            Write-Warning "Your PATH environment variable has been updated for this session. Please add 'export PATH=$($targetPath):`$PATH' to your shell's profile.  For example, ~/.bashrc for Bash."
        }
    }

    # Log into the server
    git-lfs-login --url $URL --user $User

    # Get the suffix
    if ($IsWindows) {
        $suffix = ".exe"
    }
    else {
        $suffix = ""
    }

    # We need this path for configuring Git.
    $which = Join-Path $targetPath "git-lfs-login$suffix"

    # Configure Git
    git config --global lfs.standalonetransferagent git-lfs-synology
    git config --global lfs.customtransfer.git-lfs-synology.path "$which"
}
