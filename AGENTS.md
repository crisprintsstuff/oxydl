# OxyDL Development & Release Guidelines

## Mandatory Release Workflow

Whenever a new release is prepared, built, or pushed to GitHub:

1. **Update `CHANGELOG.md`**:
   - Always update `CHANGELOG.md` following [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) standards.
   - Record the version number, release date, and categorize all changes under appropriate headers (`Added`, `Changed`, `Fixed`, `Removed`, `Security`).
   - Move relevant items out of `[Unreleased]` into the new release section.

2. **Version Synchronization**:
   - Keep version numbers synchronized across `Cargo.toml`, `crates/installer/Cargo.toml`, `extension/manifest.json`, and `CHANGELOG.md`.

3. **Release Packaging**:
   - Execute `./installer/build_release.sh` to cross-compile Windows binaries (`oxydl.exe`, `install_oxydl.exe`) and build distribution packages (`OxyDL-v*-Windows-x64-Portable.zip`, `install_oxydl.exe`, `oxydl-browser-extension.zip`).
   - Verify that `CHANGELOG.md` and `README.md` are included in the portable bundle.

4. **Git & GitHub Release**:
   - Commit all changes and push to `origin master`.
   - Update or publish the GitHub Release with matching tag, release notes, and uploaded artifacts (`install_oxydl.exe`, portable ZIP, standalone `oxydl.exe`, and browser extension ZIP).
