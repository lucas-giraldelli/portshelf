# Signing the Windows installer

The Windows installer is not signed yet, so SmartScreen warns before running it ("More info", then "Run anyway"). This is the plan to remove that warning.

## Plan

1. Build the releases on GitHub Actions: a workflow that builds the Linux AppImage and the Windows installer for every version tag and attaches both to the release.
2. Apply to the SignPath Foundation, which signs open source projects for free with its own certificate. PortShelf qualifies: a public repository, the MIT license, and builds made on CI.
3. Once approved, add the SignPath step to the Windows job so the installer is signed before it is attached to the release.

A signed installer can still get a warning for its first downloads, until it builds up reputation with Microsoft.

## Other options

- Microsoft Store: the Store signs the app, and individual developer accounts are free. It takes a Store listing and its review.
- Azure Artifact Signing (formerly Trusted Signing): around US$10 a month; when last checked, individuals had to be in the US or Canada.
- A bought code signing certificate (OV): US$200 to 400 a year, and the warning stays until the installer builds up reputation. Since 2024 EV certificates no longer skip it either.

Prices and eligibility rules change; check them again before picking one.
