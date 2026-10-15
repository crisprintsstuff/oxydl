#!/usr/bin/env bash
set -e

# OxyDL Windows Release & Packaging Script
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$DIR"

echo "=========================================================="
echo "  OxyDL Windows Release Build: Portable & Native Installer"
echo "=========================================================="
source "$HOME/.cargo/env" 2>/dev/null || true
export PATH="$HOME/.cargo/bin:$PATH"

# 1. Build main application binary (oxydl.exe)
echo ""
echo "--> [1/5] Compiling main application (oxydl.exe)..."
cargo xwin build -p oxydl --target x86_64-pc-windows-msvc --release

MAIN_EXE="$DIR/target/x86_64-pc-windows-msvc/release/oxydl.exe"
if [ ! -f "$MAIN_EXE" ]; then
    echo "Error: oxydl.exe was not found at $MAIN_EXE"
    exit 1
fi
echo "    Generated: $MAIN_EXE ($(ls -lh "$MAIN_EXE" | awk '{print $5}'))"

# 2. Compress oxydl.exe into payload for installer
echo ""
echo "--> [2/5] Packaging compressed payload for installer..."
mkdir -p "$DIR/crates/installer/payload"
gzip -k -9 -c "$MAIN_EXE" > "$DIR/crates/installer/payload/oxydl.exe.gz"
echo "    Payload size: $(ls -lh "$DIR/crates/installer/payload/oxydl.exe.gz" | awk '{print $5}')"

# 3. Build native Windows installer (install_oxydl.exe)
echo ""
echo "--> [3/5] Compiling standalone Windows installer (install_oxydl.exe)..."
cargo xwin build -p install_oxydl --target x86_64-pc-windows-msvc --release

INSTALLER_EXE="$DIR/target/x86_64-pc-windows-msvc/release/install_oxydl.exe"
if [ ! -f "$INSTALLER_EXE" ]; then
    echo "Error: install_oxydl.exe was not found at $INSTALLER_EXE"
    exit 1
fi
echo "    Generated: $INSTALLER_EXE ($(ls -lh "$INSTALLER_EXE" | awk '{print $5}'))"

# 4. Package distribution directory
echo ""
echo "--> [4/5] Finalizing distribution packages..."
DIST_DIR="$DIR/dist"
PORTABLE_DIR="$DIST_DIR/portable"
rm -rf "$PORTABLE_DIR"
mkdir -p "$PORTABLE_DIR"

# Copy Portable files
cp "$MAIN_EXE" "$PORTABLE_DIR/oxydl.exe"
cp "$DIR/README.md" "$PORTABLE_DIR/README.md"
cp "$DIR/CHANGELOG.md" "$PORTABLE_DIR/CHANGELOG.md"
touch "$PORTABLE_DIR/portable.txt"

# Create Portable ZIP archive
cd "$PORTABLE_DIR"
ZIP_NAME="OxyDL-v0.1.0-Windows-x64-Portable.zip"
rm -f "$DIST_DIR/$ZIP_NAME"
zip -r "$DIST_DIR/$ZIP_NAME" ./* > /dev/null

# Copy Installer to dist
cp "$INSTALLER_EXE" "$DIST_DIR/install_oxydl.exe"

# 5. Package Browser Extension
echo ""
echo "--> [5/5] Packaging Chrome & Firefox browser extension..."
cd "$DIR/extension"
EXT_ZIP="oxydl-browser-extension.zip"
rm -f "$DIST_DIR/$EXT_ZIP"
zip -r "$DIST_DIR/$EXT_ZIP" ./* -x "*.git*" > /dev/null
cd "$DIR"

echo ""
echo "=========================================================="
echo "  Build Complete! Distribution Artifacts Ready:          "
echo "=========================================================="
echo " 1. Native Windows Installer: $DIST_DIR/install_oxydl.exe ($(ls -lh "$DIST_DIR/install_oxydl.exe" | awk '{print $5}'))"
echo " 2. Portable ZIP Package:     $DIST_DIR/$ZIP_NAME ($(ls -lh "$DIST_DIR/$ZIP_NAME" | awk '{print $5}'))"
echo " 3. Portable Folder:          $DIST_DIR/portable/ (oxydl.exe + portable.txt)"
echo " 4. Browser Extension:        $DIST_DIR/$EXT_ZIP ($(ls -lh "$DIST_DIR/$EXT_ZIP" | awk '{print $5}'))"
echo "=========================================================="
