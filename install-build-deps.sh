#!/usr/bin/env bash
# Autodetects the Linux distribution and installs the required dependencies to build Notion-rs (Tauri)

set -e

echo "==> Detecting Linux distribution..."

if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
    LIKE=$ID_LIKE
else
    echo "❌ /etc/os-release not found. Unsupported system."
    exit 1
fi

echo "==> Detected OS: $OS"

if [[ "$OS" == "ubuntu" || "$OS" == "debian" || "$LIKE" == *"ubuntu"* || "$LIKE" == *"debian"* ]]; then
    echo "==> Updating apt and installing dependencies for Debian/Ubuntu..."
    sudo apt-get update
    sudo apt-get install -y \
        libwebkit2gtk-4.1-dev \
        build-essential curl wget file \
        libxdo-dev libssl-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev \
        libhunspell-dev \
        libclang-dev clang llvm

elif [[ "$OS" == "fedora" || "$LIKE" == *"fedora"* || "$LIKE" == *"rhel"* ]]; then
    echo "==> Installing dependencies for Fedora/RHEL..."
    sudo dnf install -y \
        webkit2gtk4.1-devel \
        gcc-c++ curl wget file \
        libxdo-devel openssl-devel \
        libappindicator-gtk3-devel \
        librsvg2-devel \
        hunspell-devel \
        clang llvm-devel

elif [[ "$OS" == "arch" || "$LIKE" == *"arch"* ]]; then
    echo "==> Installing dependencies for Arch Linux..."
    sudo pacman -Syu --noconfirm \
        webkit2gtk-4.1 \
        base-devel curl wget file \
        xdotool openssl \
        libappindicator-gtk3 \
        librsvg \
        hunspell \
        clang llvm

elif [[ "$OS" == "alpine" ]]; then
    echo "==> Installing dependencies for Alpine Linux..."
    sudo apk add \
        webkit2gtk-dev \
        build-base curl wget file \
        xdotool-dev openssl-dev \
        libappindicator-dev \
        librsvg-dev \
        hunspell-dev \
        clang llvm

else
    echo "❌ Unsupported or unrecognized distribution: $OS"
    echo "Please check the Tauri prerequisites documentation for your OS: https://tauri.app/v1/guides/getting-started/prerequisites"
    exit 1
fi

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "==> Rust is not installed. Installing Rustcast/Cargo..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    echo "==> Rust installed successfully!"
    echo "==> Please source your environment or restart your terminal: source \$HOME/.cargo/env"
else
    echo "==> Rust is already installed: $(cargo --version)"
fi

echo "==> ✅ All development dependencies installed successfully!"
