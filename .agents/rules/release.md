---
trigger: always_on
description: Always update CHANGELOG.md whenever preparing or pushing a new release
---

# Release Rule: Keep CHANGELOG.md Updated

Whenever a new release or release build is prepared, tagged, or pushed to GitHub:
- Check and update `CHANGELOG.md` with the release version, date, and detailed categorized changes (`Added`, `Changed`, `Fixed`, `Removed`).
- Ensure the portable distribution archive (`dist/OxyDL-*-Windows-x64-Portable.zip`) includes the updated `CHANGELOG.md`.
- Keep release notes on GitHub Releases aligned with the entries in `CHANGELOG.md`.
