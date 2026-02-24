# TauriTidal

Still in VERY early development. Expect bugs.

A native desktop player for Tidal, built with Tauri v2, React, and Rust.

TauriTidal connects directly to the Tidal API to stream lossless audio through a native Rust audio pipeline. It integrates with macOS media keys and Now Playing, runs as a lightweight native app (not Electron), and gives you full control over your library, playlists, and queue.

<img width="1187" height="793" alt="Screenshot 2026-02-24 at 8 35 08 AM" src="https://github.com/user-attachments/assets/5db57d19-c398-4455-a6e3-a45218a91e4b" />

## Architecture

```
┌──────────────────────────────────────────────────┐
│  React Frontend (TypeScript)                     │
│  ┌────────────┐ ┌──────────┐ ┌────────────────┐  │
│  │ Pages      │ │ Hooks    │ │ Stores         │  │
│  │ Search     │ │ useAuth  │ │ playerStore    │  │
│  │ Library    │ │ usePlay  │ │ libraryStore   │  │
│  │ Playlist   │ │ useLib   │ │ authStore      │  │
│  │ Album      │ │ useSearch│ │ queueStore     │  │
│  │ Artist     │ │          │ │ searchStore    │  │
│  │ Favorites  │ │          │ │ uiStore        │  │
│  │ Queue      │ │          │ │                │  │
│  │ Similar    │ │          │ │                │  │
│  └────────────┘ └──────────┘ └────────────────┘  │
│          Tauri IPC Commands + Events             │
├──────────────────────────────────────────────────┤
│  Rust Backend                                    │
│  ┌────────────┐ ┌──────────┐ ┌────────────────┐  │
│  │ API Layer  │ │ Audio    │ │ macOS Native   │  │
│  │ client     │ │ player   │ │ now_playing    │  │
│  │ auth       │ │ decoder  │ │ media_keys     │  │
│  │ search     │ │ stream   │ │                │  │
│  │ tracks     │ │ queue    │ │                │  │
│  │ playlists  │ │ preloader│ │                │  │
│  │ albums     │ │          │ │                │  │
│  │ artists    │ │          │ │                │  │
│  │ favorites  │ │          │ │                │  │
│  └────────────┘ └──────────┘ └────────────────┘  │
└──────────────────────────────────────────────────┘
```

**Frontend**: React 18 with Zustand for state management, shadcn/ui components, Tailwind CSS v4, and React Router for navigation. Communicates with the backend exclusively through typed Tauri IPC commands and event listeners.

**Backend**: Rust with Symphonia for audio decoding, cpal for audio output, and reqwest for HTTP. Handles all Tidal API communication, audio streaming, and macOS system integration through objc2.

For a detailed walkthrough of the audio pipeline, state management, and IPC surface, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Features

**Playback**
- Lossless (FLAC) and AAC streaming from Tidal
- Gapless-ready audio pipeline with track preloading
- Play, pause, stop, next, previous, seek
- Volume control with mute toggle, both persisted across sessions
- Queue management with shuffle, repeat (off/all/one), and drag-to-reorder

**Library**
- Search tracks, albums, artists, and playlists with debounced suggestions
- Browse and manage playlists (create, delete, add/remove tracks)
- Favorites with add/remove toggle and cursor-based pagination
- Album and artist detail pages with full discography
- Similar tracks discovery from any track
- Personalized recommendations on the home page

**macOS Integration**
- Media key support (play/pause, next, previous) via MPRemoteCommandCenter
- Now Playing info in Control Center with track title, artist, album art, and elapsed time
- Native window with 1200x800 default, 800x500 minimum

**Auth**
- OAuth device code flow for login (primary)
- PKCE with deep-link callback as fallback
- Client credentials mode for unauthenticated catalog browsing
- Automatic token refresh on 401 responses
- Credentials stored locally in `~/.tauritidal/config.json`

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Framework | Tauri v2 |
| Frontend | React 18, TypeScript, Vite 6 |
| Styling | Tailwind CSS v4 (CSS-first config), shadcn/ui (Radix primitives) |
| State | Zustand 5 |
| Routing | React Router v7 |
| Backend | Rust (2021 edition) |
| Audio decode | Symphonia (FLAC, AAC, MP3, WAV, OGG) |
| Audio output | cpal |
| HTTP | reqwest with rustls-tls |
| macOS native | objc2 0.6, objc2-media-player, objc2-foundation |
| Drag and drop | dnd-kit |
| Icons | Lucide React |

## Prerequisites

- macOS (required for media key and Now Playing integration)
- [Rust](https://rustup.rs/) (stable)
- [Bun](https://bun.sh/) (used as the JS package manager and dev server)
- A Tidal account (HiFi or HiFi Plus for lossless)

## Setup

```bash
# Clone
git clone https://github.com/Kadajett/Tauri-Tidal.git
cd Tauri-Tidal

# Install frontend dependencies
bun install

# Run in dev mode (starts both Vite dev server and Tauri)
bun run tauri dev

# Build for release
bun run tauri build
```

The release build produces both a `.app` bundle and a `.dmg` installer in `src-tauri/target/release/bundle/`.

On first launch, the app prompts you to log in via Tidal's device code flow. Open the displayed URL, enter the code, and authorize the app. Without logging in, you can still browse the Tidal catalog using client credentials (30-second previews only).

## Project Structure

```
src/                              # React frontend
  App.tsx                         # Route definitions
  index.css                       # Tailwind v4 theme (dark, OKLch colors)
  components/
    player/                       # FooterPlayer, PlayerControls, ProgressBar,
                                  # VolumeControl, TrackInfo
    layout/                       # AppLayout, Sidebar
    track/                        # TrackList, TrackRow (with context menu)
    cards/                        # AlbumCard, ArtistCard, PlaylistCard
    search/                       # SearchBar, SearchResults
    ui/                           # shadcn/ui primitives (Button, Dialog, Slider, etc.)
  pages/                          # SearchPage, LibraryPage, PlaylistPage, AlbumPage,
                                  # ArtistPage, FavoritesPage, QueuePage, SimilarTracksPage
  hooks/                          # useAuth, usePlayback, useLibrary, useSearch,
                                  # useKeyboardShortcuts, useAnimatedProgress, useTauriEvent
  stores/                         # playerStore, authStore, libraryStore, queueStore,
                                  # searchStore, uiStore
  types/                          # TypeScript type definitions (api, track, events,
                                  # player, search)
  lib/
    tauri.ts                      # Typed IPC wrappers for all Tauri commands
    constants.ts                  # Route paths, keyboard shortcut mappings
    utils.ts                      # cn() class merging, formatTime()

src-tauri/                        # Rust backend
  src/
    api/                          # Tidal API client: auth, client, search, tracks,
                                  # albums, artists, playlists, user, models
    audio/                        # Audio pipeline: player, decoder, stream_source,
                                  # queue, preloader
    commands/                     # Tauri IPC handlers: auth, playback, queue, search,
                                  # playlist, favorites, browse
    macos/                        # Now Playing and media key handlers via objc2
    config.rs                     # App config persistence (~/.tauritidal/config.json)
    events.rs                     # Event payload types for frontend communication
    error.rs                      # Unified error type (AppError enum)
    lib.rs                        # Tauri app setup, AppState, command registration
  tauri.conf.json                 # Tauri config: window, CSP, deep-link, bundle
  capabilities/                   # Security permission scopes

docs/
  tidal-api-oas.json              # Tidal API OpenAPI spec (reference)
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Space | Play / Pause |
| Left Arrow | Seek backward |
| Right Arrow | Seek forward |
| Up Arrow | Volume up |
| Down Arrow | Volume down |
| `n` | Next track |
| `p` | Previous track |
| `m` | Toggle mute |
| `/` | Focus search bar |

## Configuration

App config is stored at `~/.tauritidal/config.json` and includes:

- Auth tokens (access token, refresh token, expiration)
- User profile (user ID, display name, country code)
- Audio quality preference (default: `LOSSLESS`)
- Volume and mute state

Queue state persists separately in `~/.tauritidal/queue.json`, including track order, current position, shuffle state, and repeat mode. Both files are created automatically on first use.

## Development

```bash
# Dev mode with hot reload
bun run tauri dev

# TypeScript type check
bunx tsc --noEmit

# Rust check (from repo root)
cargo check --manifest-path src-tauri/Cargo.toml

# Production build
bun run tauri build
```

The Vite dev server runs on port 1420 with HMR on 1421. Tauri watches for frontend changes automatically. Rust backend changes trigger a recompile.

## Known Limitations

- macOS only (objc2 bindings for Now Playing and media keys are macOS-specific)
- No offline/download support
- No lyrics display (API model exists but UI is not wired up)
- No EQ or audio effects

## License

MIT
