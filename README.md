# TauriTidal

Still in VERY early development. Expect bugs.

A native desktop player for Tidal, built with Tauri v2, React, and Rust.

TauriTidal connects directly to the Tidal API to stream lossless audio through a native Rust audio pipeline. It integrates with macOS media keys and Now Playing, runs as a lightweight native app (not Electron), and gives you full control over your library, playlists, and queue.

<img width="1187" height="793" alt="Screenshot 2026-02-24 at 8 35 08 AM" src="https://github.com/user-attachments/assets/5db57d19-c398-4455-a6e3-a45218a91e4b" />


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
│  │ Favorites  │ │          │ │                │  │
│  │ Queue      │ │          │ │                │  │
│  └────────────┘ └──────────┘ └────────────────┘  │
│                Tauri IPC Commands                 │
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
│  │ user       │ │          │ │                │  │
│  └────────────┘ └──────────┘ └────────────────┘  │
└──────────────────────────────────────────────────┘
```

**Frontend**: React 18 with Zustand for state management, shadcn/ui components, Tailwind CSS v4, and React Router for navigation. Communicates with the backend exclusively through Tauri IPC commands.

**Backend**: Rust with Symphonia for audio decoding, cpal for audio output, and reqwest for HTTP. Handles all Tidal API communication, audio streaming, and macOS system integration through objc2.

## Features

**Playback**
- Lossless (FLAC) and AAC streaming from Tidal
- Gapless-ready audio pipeline with track preloading
- Play, pause, stop, next, previous, seek
- Volume control with persistence
- Queue management with shuffle, repeat (off/all/one), and drag-to-reorder

**Library**
- Search tracks, albums, artists, and playlists
- Browse and manage playlists (create, delete, add/remove tracks)
- Favorites with add/remove toggle
- Album and artist detail pages
- Discovery recommendations based on favorites and mixes

**macOS Integration**
- Media key support (play/pause, next, previous) via MPRemoteCommandCenter
- Now Playing info in Control Center with track title, artist, album art
- Native window with 1200x800 default, 800x500 minimum

**Auth**
- OAuth device code flow for login
- Automatic token refresh
- Credentials stored locally in `~/.mactidal/config.json`

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Framework | Tauri v2 |
| Frontend | React 18, TypeScript, Vite |
| Styling | Tailwind CSS v4, shadcn/ui (Radix primitives) |
| State | Zustand |
| Routing | React Router v7 |
| Backend | Rust (2021 edition) |
| Audio decode | Symphonia (FLAC, AAC, MP3, WAV, OGG) |
| Audio output | cpal |
| HTTP | reqwest with rustls-tls |
| macOS native | objc2, objc2-media-player, objc2-foundation |
| Drag & drop | dnd-kit |

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

On first launch, the app will prompt you to log in via Tidal's device code flow. Open the displayed URL, enter the code, and authorize the app.

## Project Structure

```
src/                          # React frontend
  components/
    player/                   # FooterPlayer, PlayerControls, ProgressBar, VolumeControl, TrackInfo
    layout/                   # AppLayout, Sidebar
    track/                    # TrackList, TrackRow
    cards/                    # AlbumCard, ArtistCard, PlaylistCard
    search/                   # SearchBar, SearchResults
    ui/                       # shadcn/ui primitives (Button, Dialog, Slider, etc.)
  pages/                      # SearchPage, LibraryPage, PlaylistPage, AlbumPage, ArtistPage,
                              # FavoritesPage, QueuePage, SimilarTracksPage
  hooks/                      # useAuth, usePlayback, useLibrary, useSearch, useKeyboardShortcuts,
                              # useAnimatedProgress, useTauriEvent
  stores/                     # playerStore, authStore, libraryStore, queueStore, searchStore, uiStore
  types/                      # TypeScript type definitions (api, track, events)
  lib/                        # Tauri IPC wrappers

src-tauri/                    # Rust backend
  src/
    api/                      # Tidal API client: auth, search, tracks, albums, artists, playlists, user
    audio/                    # Audio pipeline: player, decoder, stream_source, queue, preloader
    commands/                 # Tauri IPC command handlers (one file per domain)
    macos/                    # Now Playing info and media key handlers via objc2
    config.rs                 # App config (~/.mactidal/config.json)
    events.rs                 # Event payload types for frontend communication
    error.rs                  # Unified error type
    lib.rs                    # Tauri app setup and command registration
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Space | Play / Pause |
| Left Arrow | Seek backward |
| Right Arrow | Seek forward |
| Up Arrow | Volume up |
| Down Arrow | Volume down |
| `/` | Focus search bar |

## Configuration

App config is stored at `~/.mactidal/config.json` and includes auth tokens, audio quality preference, and volume state. Queue state persists across sessions in the same directory.

## Known Limitations

- macOS only (objc2 bindings for Now Playing and media keys are macOS-specific)
- Seeking/scrubbing is partially implemented; full byte-range seeking in streaming mode is a work in progress
- No offline/download support
- No lyrics display (API model exists but UI is not wired up)

## License

MIT
