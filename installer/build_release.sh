#!/usr/bin/env bash
set -e

# Oxidm Windows Release & Packaging Script
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$DIR"

echo "=========================================================="
echo "  Oxidm Windows Release Build: Portable & Native Installer "
echo "=========================================================="
source "$HOME/.cargo/env" 2>/dev/null || true
export PATH="$HOME/.cargo/bin:$PATH"

# 1. Build main application binary (oxidm.exe)
echo ""
echo "--> [1/4] Compiling main application (oxidm.exe)..."
cargo xwin build -p oxidm --target x86_64-pc-windows-msvc --release

MAIN_EXE="$DIR/target/x86_64-pc-windows-msvc/release/oxidm.exe"
if [ ! -f "$MAIN_EXE" ]; then
    echo "Error: oxidm.exe was not found at $MAIN_EXE"
    exit 1
fi
echo "    Generated: $MAIN_EXE ($(ls -lh "$MAIN_EXE" | awk '{print $5}'))"

# 2. Compress oxidm.exe into payload for installer
echo ""
echo "--> [2/4] Packaging compressed payload for installer..."
mkdir -p "$DIR/crates/installer/payload"
gzip -k -9 -c "$MAIN_EXE" > "$DIR/crates/installer/payload/oxidm.exe.gz"
echo "    Payload size: $(ls -lh "$DIR/crates/installer/payload/oxidm.exe.gz" | awk '{print $5}')"

# 3. Build native Windows installer (install_oxidm.exe)
echo ""
echo "--> [3/4] Compiling standalone Windows installer (install_oxidm.exe)..."
cargo xwin build -p install_oxidm --target x86_64-pc-windows-msvc --release

INSTALLER_EXE="$DIR/target/x86_64-pc-windows-msvc/release/install_oxidm.exe"
if [ ! -f "$INSTALLER_EXE" ]; then
    echo "Error: install_oxidm.exe was not found at $INSTALLER_EXE"
    exit 1
fi
echo "    Generated: $INSTALLER_EXE ($(ls -lh "$INSTALLER_EXE" | awk '{print $5}'))"

# 4. Package distribution directory
echo ""
echo "--> [4/4] Finalizing distribution packages..."
DIST_DIR="$DIR/dist"
PORTABLE_DIR="$DIST_DIR/portable"
rm -rf "$PORTABLE_DIR"
mkdir -p "$PORTABLE_DIR"

# Copy Portable files
cp "$MAIN_EXE" "$PORTABLE_DIR/oxidm.exe"
cp "$DIR/README.md" "$PORTABLE_DIR/README.md"
touch "$PORTABLE_DIR/portable.txt"

# Create Portable ZIP archive
cd "$PORTABLE_DIR"
ZIP_NAME="Oxidm-v0.1.0-Windows-x64-Portable.zip"
rm -f "$DIST_DIR/$ZIP_NAME"
zip -r "$DIST_DIR/$ZIP_NAME" ./* > /dev/null

# Copy Installer to dist
cp "$INSTALLER_EXE" "$DIST_DIR/install_oxidm.exe"

echo ""
echo "=========================================================="
echo "  Build Complete! Distribution Artifacts Ready:          "
echo "=========================================================="
echo " 1. Native Windows Installer: $DIST_DIR/install_oxidm.exe ($(ls -lh "$DIST_DIR/install_oxidm.exe" | awk '{print $5}'))"
echo " 2. Portable ZIP Package:     $DIST_DIR/$ZIP_NAME ($(ls -lh "$DIST_DIR/$ZIP_NAME" | awk '{print $5}'))"
echo " 3. Portable Folder:          $DIST_DIR/portable/ (oxidm.exe + portable.txt)"
echo "=========================================================="
