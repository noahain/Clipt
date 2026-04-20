#!/usr/bin/env python3
"""
Build script to create Clipt executable
"""

import PyInstaller.__main__
from pathlib import Path
import shutil
import os

# Get current directory
current_dir = Path(__file__).parent

# Define paths
main_script = current_dir / "main.py"
icon_file = current_dir / "assets" / "icon.ico"
ui_dir = current_dir / "ui"
assets_dir = current_dir / "assets"

# Build command arguments
args = [
    str(main_script),
    '--name=Clipt',
    '--onefile',
    '--windowed',
    f'--icon={icon_file}',
    '--add-data', f'{ui_dir};ui',
    '--add-data', f'{assets_dir};assets',
    '--clean',
    '--noconfirm',
]

print("Building Clipt executable...")
print(f"Icon: {icon_file}")
print(f"UI dir: {ui_dir}")
print(f"Assets dir: {assets_dir}")

PyInstaller.__main__.run(args)

print("\nBuild complete!")
print("Executable location: dist/Clipt.exe")
