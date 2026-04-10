# PDF Watermark

PDF Watermark is a lightweight desktop app built with Tauri and Vite for adding tiled text watermarks to PDF files.

## Features

- Choose a local PDF file from a native file picker
- Enter custom watermark text
- Save the processed file to a new PDF path
- View page-by-page processing progress in the UI

## Tech Stack

- Frontend: Vite + vanilla JavaScript
- Desktop runtime: Tauri 2
- Native dialog support: `@tauri-apps/plugin-dialog`

## Requirements

- Node.js 18+
- Rust toolchain
- Tauri development prerequisites for your platform

## Getting Started

Install dependencies:

```bash
npm install
```

Run the app in development mode:

```bash
npm run tauri dev
```

Build the frontend bundle:

```bash
npm run build
```

Build the desktop application:

```bash
npm run tauri build
```

## GitHub Actions Build

The repository includes `.github/workflows/build.yml` for building installers on GitHub-hosted runners.

- `windows-latest` builds Windows installers
- `macos-latest` builds macOS installers
- You can trigger it manually from the Actions tab
- Pushing a tag like `v0.1.0` also triggers a build

Build outputs are uploaded as workflow artifacts from `src-tauri/target/release/bundle/`.

Note: the workflow creates unsigned builds. If you want notarized macOS apps or signed Windows installers, add the required signing certificates and secrets in GitHub Actions later.

## GitHub Actions Build

The repository includes `.github/workflows/build.yml` for building installers on GitHub-hosted runners.

- `windows-latest` builds Windows installers
- `macos-latest` builds macOS installers
- You can trigger it manually from the Actions tab
- Pushing a tag like `v0.1.0` also triggers a build

Build outputs are uploaded as workflow artifacts from `src-tauri/target/release/bundle/`.

Note: the workflow creates unsigned builds. If you want notarized macOS apps or signed Windows installers, add the required signing certificates and secrets in GitHub Actions later.

## How To Use

1. Click `Choose PDF` to select the source file.
2. Edit the watermark text if needed.
3. Click `Save As` to choose the output location, or keep the suggested path.
4. Click `Add Watermark` to generate the new PDF.

## Project Structure

- `index.html`: app entry HTML
- `src/main.js`: UI logic and Tauri integration
- `src/styles.css`: application styles
- `src-tauri/`: Rust backend and Tauri configuration
