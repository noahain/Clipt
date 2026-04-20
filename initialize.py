#!/usr/bin/env python3
"""
Clipt Initialization Script
Checks dependencies, environment setup, and creates necessary directories
"""

import os
import sys
import subprocess
from pathlib import Path


def check_python_version():
    """Check if Python version is 3.8+"""
    version = sys.version_info
    if version.major < 3 or (version.major == 3 and version.minor < 8):
        print("❌ Python 3.8 or higher is required")
        return False
    print(f"✓ Python {version.major}.{version.minor}.{version.micro}")
    return True


def check_dependencies():
    """Check if required packages are installed"""
    required = [
        'pywebview',
        'pystray',
        'PIL',
        'openai',
        'dotenv',
        'pyperclip'
    ]

    missing = []
    for package in required:
        try:
            __import__(package)
            print(f"✓ {package}")
        except ImportError:
            missing.append(package)
            print(f"✗ {package} (missing)")

    return missing


def install_dependencies(missing):
    """Install missing dependencies"""
    print("\n📦 Installing dependencies...")
    try:
        subprocess.check_call([sys.executable, '-m', 'pip', 'install', '-r', 'requirements.txt'])
        print("✓ Dependencies installed successfully")
        return True
    except subprocess.CalledProcessError as e:
        print(f"❌ Failed to install dependencies: {e}")
        return False


def setup_environment():
    """Check and create .env file"""
    env_path = Path('.env')
    env_example = Path('.env.example')

    if env_path.exists():
        # Check if NVIDIA_API_KEY is set
        with open(env_path, 'r') as f:
            content = f.read()
            if 'NVIDIA_API_KEY=' in content and 'your_api_key_here' not in content:
                print("✓ Environment file configured")
                return True

    print("⚠️  Environment file needs configuration")

    if not env_example.exists():
        # Create template
        with open(env_example, 'w') as f:
            f.write("NVIDIA_API_KEY=your_api_key_here\n")

    # Copy template
    with open(env_example, 'r') as src:
        with open(env_path, 'w') as dst:
            dst.write(src.read())

    print("📝 Created .env file from template")
    print("   Please edit .env and add your NVIDIA API key")
    print("   Get your API key from: https://developer.nvidia.com/")
    return False


def create_directories():
    """Create necessary directories"""
    Path('Days').mkdir(exist_ok=True)
    print("✓ Directories created")
    return True


def print_banner():
    """Print startup banner"""
    print("""
  ██████╗██╗     ██╗██████╗ ████████╗
 ██╔════╝██║     ██║██╔══██╗╚══██╔══╝
 ██║     ██║     ██║██████╔╝   ██║
 ██║     ██║     ██║██╔═══╝    ██║
 ╚██████╗███████╗██║██║        ██║
  ╚═════╝╚══════╝╚═╝╚═╝        ╚═╝

 Clipboard History Manager with AI Chat
 ======================================\n""")


def main():
    """Main initialization function"""
    print_banner()

    print("🔍 Checking requirements...\n")

    # Check Python version
    if not check_python_version():
        sys.exit(1)

    # Check dependencies
    missing = check_dependencies()
    if missing:
        print("\n📦 Some dependencies are missing")
        response = input("Would you like to install them now? (y/n): ")
        if response.lower() == 'y':
            if not install_dependencies(missing):
                sys.exit(1)
        else:
            print("❌ Cannot continue without dependencies")
            sys.exit(1)

    # Setup environment
    env_ready = setup_environment()

    # Create directories
    create_directories()

    print("\n" + "="*40)

    if env_ready:
        print("✅ Initialization complete!")
        print("\n🚀 Starting Clipt...")
        return 0
    else:
        print("⚠️  Initialization incomplete")
        print("\nPlease:")
        print("1. Edit .env file")
        print("2. Add your NVIDIA_API_KEY")
        print("3. Run: python main.py")
        return 1


if __name__ == '__main__':
    sys.exit(main())
