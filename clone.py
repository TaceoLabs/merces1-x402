#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import requests
import os
import shutil
import zipfile
import io

REPO_NAME = "Merces-46CE"
DESTINATION = "Merces"
ZIP_URL = "https://anonymous.4open.science/api/repo/" + REPO_NAME + "/zip"

def download_folder():
    print("Downloading zip file from " + ZIP_URL)
    r = requests.get(ZIP_URL)
    z = zipfile.ZipFile(io.BytesIO(r.content))
    print("Extracting zip file to " + DESTINATION)
    z.extractall(DESTINATION)

def get_submodules():
    os.chdir(DESTINATION)
    os.chdir("contracts")
    if os.path.exists("lib"):
        shutil.rmtree("lib")
    os.mkdir("lib")
    os.chdir("lib")
    os.system("git clone https://github.com/OpenZeppelin/openzeppelin-contracts")
    os.system("git clone https://github.com/foundry-rs/forge-std")
    os.system("git clone https://github.com/TaceoLabs/babyjubjub-solidity")

    os.chdir("openzeppelin-contracts")
    os.system("git checkout 8ff78ffb6e78463f070eab59487b4ba30481b53c")
    os.chdir("../forge-std")
    os.system("git checkout 0e44f85a13976ba7491c6a9ee994b1a7efc3c281")
    os.chdir("../babyjubjub-solidity")
    os.system("git checkout 4202e8794c44cf3f894c910354c0567b659e6669")
    os.chdir("../..")

def main():
    # Remove folder if it exists
    path = os.getcwd()
    des_path = path + "/" + DESTINATION
    if os.path.exists(des_path):
        shutil.rmtree(des_path)


    # download zip file and extract
    download_folder()

    # Clone submodules
    get_submodules()


if __name__ == "__main__":
    main()
