#!/usr/bin/env bash
# Quick install script for Lotion-rs
# Fetches the latest release from GitHub and installs it locally based on the OS and distribution.

set -e

REPO="YoannDev90/lotion-rs"
BASE_URL="https://github.com/$REPO/releases/latest/download"

echo "==> Detecting OS..."
OS="$(uname -s)"
ARCH="$(uname -m)"

if [ "$OS" = "Linux" ]; then
    if [ "$ARCH" != "x86_64" ] && [ "$ARCH" != "amd64" ]; then
        echo "⚠️  Warning: Pre-built binaries are mainly tested on x86_64/amd64. You are on $ARCH."
    fi

    if [ -f /etc/os-release ]; then
        . /etc/os-release
        DIST=$ID
        LIKE=$ID_LIKE
    else
        DIST="unknown"
    fi

    if [[ "$DIST" == "ubuntu" || "$DIST" == "debian" || "$LIKE" == *"ubuntu"* || "$LIKE" == *"debian"* ]]; then
        echo "==> Detected Debian/Ubuntu based OS. Downloading .deb package..."
        FILE="lotion-rs_amd64.deb"
        curl -fsSL -o "$FILE" "$BASE_URL/$FILE"
        echo "==> Installing $FILE (requires sudo)..."
        # Using apt-get install instead of dpkg -i to automatically resolve missing runtime dependencies
        sudo apt-get install -y "./$FILE"
        rm -f "$FILE"
        echo "==> ✅ Lotion-rs installed successfully!"
        exit 0

    elif [[ "$DIST" == "fedora" || "$LIKE" == *"fedora"* || "$LIKE" == *"rhel"* || "$DIST" == "centos" ]]; then
        echo "==> Detected Fedora/RHEL based OS. Downloading .rpm package..."
        FILE="lotion-rs.rpm"
        curl -fsSL -o "$FILE" "$BASE_URL/$FILE"
        echo "==> Installing $FILE (requires sudo)..."
        if command -v dnf &> /dev/null; then
            sudo dnf install -y "./$FILE"
        else
            sudo yum install -y "./$FILE"
        fi
        rm -f "$FILE"
        echo "==> ✅ Lotion-rs installed successfully!"
        exit 0

    else
        echo "==> Using fallback: Downloading AppImage..."
        FILE="lotion-rs.AppImage"
        curl -fsSL -o "$FILE" "$BASE_URL/$FILE"
        chmod +x "$FILE"
        echo "==> Installing AppImage to ~/.local/bin/lotion-rs ..."
        mkdir -p "$HOME/.local/bin"
        mv "$FILE" "$HOME/.local/bin/lotion-rs"
        
        echo "==> ✅ Lotion-rs installed successfully!"
        echo "==> You can now run it by typing 'lotion-rs' in your terminal (make sure ~/.local/bin is in your PATH)."
        exit 0
    fi

elif [ "$OS" = "Darwin" ]; then
    echo "==> Detected macOS. Downloading .dmg package..."
    if [ "$ARCH" = "arm64" ]; then
        FILE="lotion-rs-arm64.dmg"
    else
        FILE="lotion-rs-x64.dmg"
    fi
    
    curl -fsSL -o "$FILE" "$BASE_URL/$FILE"
    echo "==> Mounting $FILE and copying app to /Applications..."
    hdiutil attach "$FILE" -mountpoint /Volumes/LotionRs -nobrowse -quiet
    cp -R "/Volumes/LotionRs/lotion-rs.app" /Applications/ || cp -R "/Volumes/LotionRs/Lotion.app" /Applications/ || echo "⚠️ Could not auto-copy the app. Please check /Volumes/LotionRs"
    hdiutil detach /Volumes/LotionRs -quiet
    rm -f "$FILE"
    echo "==> ✅ Lotion-rs installed successfully to /Applications!"
    exit 0

else
    echo "❌ Unsupported OS: $OS. Please install manually."
    exit 1
fi
