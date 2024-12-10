#! /usr/bin/python3

import urllib.request
import json
import datetime
import platform
import os
import shutil
import zipfile

from pathlib import Path

def main():
    # Get all releases
    releases = json.load(urllib.request.urlopen("https://api.github.com/repos/ucsd-e4e/git-lfs-synology/releases"))

    # Get the latest release
    max_date = max(datetime.datetime.fromisoformat(r["published_at"]) for r in releases)
    latest_release = [r for r in releases if datetime.datetime.fromisoformat(r["published_at"]) == max_date][0]

    # Adjust the platform and architecture for the downloaded package.
    if platform.system() == "Windows":
        target_platform = "win"
    elif platform.system() == "Linux":
        target_platform = "linux"
    elif platform.system() == "Darwin":
        target_platform = "osx"
    else:
        target_platform = platform.system()

    if platform.machine() == "AMD64":
        arch = "x86_64"
    else:
        arch = platform.machine()

    # Construct the asset name.
    asset_name = f"git-lfs-synology.{target_platform}-{arch}.zip"

    # Get the Asset
    asset = [a for a in latest_release["assets"] if a["name"] == asset_name][0]

    # Get the target directory
    home_directory = Path.home()
    gitLfsSynologyDirectory = ".git-lfs-synology"
    target_path = home_directory / gitLfsSynologyDirectory

    if target_path.exists:
        shutil.rmtree(target_path)
    target_path.mkdir(parents=True)

    current_dir = os.getcwd()
    os.chdir(target_path)

    # Get the executable
    urllib.request.urlretrieve(asset["browser_download_url"], asset["name"])
    with zipfile.ZipFile(asset["name"], 'r') as zip_ref:
        zip_ref.extractall(".")

    os.chdir(current_dir)

    # Update the Path Environment Variable
    if target_path.as_posix() not in os.environ["PATH"]:
        if target_path == "win":
            seperator = ";"
        else:
            seperator = ":"

        os.environ["PATH"] = f"{target_path}{seperator}{os.environ["PATH"]}"

        # TODO: Update system PATH environment variable

    # TODO Login and Configure.
    

if __name__ == "__main__":
    main()
