# SuiVision

[中文说明](README.zh-CN.md)

SuiVision is a local-first desktop app for searching video-related resource links across multiple public disk-search sources. The first version focuses on a personal desktop workflow on Windows and macOS, with implementation centered on the desktop frontend and Tauri desktop shell. Mobile and server directories are reserved but not implemented.

## Structure

```text
SuiVision/
├── desktop-client/   # Vue 3 + TypeScript desktop frontend
├── tauri-desktop/    # Tauri 2 desktop shell and Rust search adapters
├── mobile-client/    # Reserved mobile client directory
├── server/           # Reserved server directory
├── docs/             # Development and search-rule notes
└── assets/           # Shared brand assets
```

## Local Development

Install frontend dependencies:

```bash
cd desktop-client
npm install
```

Install Tauri dependencies and start the desktop app:

```bash
cd ../tauri-desktop
npm install
npm run tauri:dev
```

Build the desktop app:

```bash
cd tauri-desktop
npm run tauri:build
```

`tauri:dev` starts the Vite frontend automatically. The frontend dev server runs on `http://127.0.0.1:5173`, and the Tauri window loads that address in development.

## Features

- Desktop search workbench with a main keyword box, source selection, disk-type filter, and exact-match option.
- Local sign-in caches only the username, display name, and login time with no expiration; sign-out or clearing app data requires signing in again.
- Media entity recognition that normalizes mixed keywords into more accurate media search terms before resource search.
- Built-in Rust search adapters for public page parsing, result normalization, and link handoff.
- Four-layer resource pool: PanSou endpoint pool, CMS V10 source pool, Torznab/Newznab indexers, and local rule plugins under `rules/sources/*.json`.
- Built-in public page sources now include PanQuDuo and several disk-search pages in addition to Hunhepan, Pikasoo, and Aliso.
- PanSou supports multiple user-configured endpoints plus `src`, channels, plugins, cloud types, cache, and refresh options.
- CMS V10 supports source-pool import and health checks. Torznab/Newznab is limited to user-owned or authorized indexers and only performs search/link handoff.
- Concurrent search across enabled sources. A failed source does not block results from other sources.
- Source coverage panel showing returned counts, failures, and disabled sources by pool type.
- Search summary shows target-TV disk-resource matches, so noisy keyword matches are not treated as success.
- Result groups for high-confidence, possible, and low-confidence matches, with ranking reasons.
- Result detail modal for resolving, copying, and opening the jump URL.
- Settings for a self-hosted PanSou endpoint, CMS V10 endpoints, and a reserved TMDB API Key field.
- Brand icon uses a dark rounded base, circular motion, and video play plus film-frame metaphors.

## Current Boundaries

- The first version is a local personal desktop app and does not connect to a private server.
- Authentication is local to the desktop client; v1 includes no server-side account system, mobile app implementation, or server-side persistence.
- The local sign-in cache stores no password, token, or expiration value. Signing out is the only in-app way to clear it.
- The app does not store user disk accounts, cookies, or private tokens.
- The app does not ship random public PanSou services, CMS sources, trackers, or indexers. Built-in public page sources only perform search, parsing, and link handoff.
- SuiVision only discovers, indexes, and opens user-visible links. It does not download content, bypass payment, DRM, or access controls.
- Search quality depends on external public sites. Network blocking, anti-crawling rules, HTML changes, and source downtime may affect results.
- The TMDB API Key field enables optional `search/multi` media-candidate enrichment. Without a key, local media rules and resource sources still work.

## Technology Stack

- Tauri 2
- Rust
- Vue 3
- TypeScript
- Vite
- Ant Design Vue
- Lucide Vue Next

## Platform Notes

- Windows and macOS are the first desktop targets through Tauri.
- The Tauri bundle target is configured as `all`, but each platform package still needs verification on the target operating system.
- Windows packaging needs separate checks for unsigned app warnings, installer behavior, and browser-link opening.
- macOS packaging needs separate checks for DMG installation, permissions, and signing or notarization if distributed outside a private environment.
