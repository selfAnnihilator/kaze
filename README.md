# Kaze — Local Music Player

A **free, open-source desktop music player** for people who own their music. No subscriptions, no cloud accounts, no tracking. Just your files, played beautifully.

> **Download:** [github.com/selfAnnihilator/kaze/releases/latest](https://github.com/selfAnnihilator/kaze/releases/latest)

---

## What it does

Kaze scans the music folders you point it at and gives you a clean, fast interface to browse, search, and play everything in your library. The longer you use it, the better it understands your taste — surfacing forgotten albums, building smart mixes, and recommending things you haven't played in a while.

Your local library works without an internet connection. Online streaming and download-source search require access to external providers.

Kaze is mainly for playing music you already have locally, with online streaming as an extra option. Download search shows possible sources for you to review, but a result may be a different recording, and downloads are not guaranteed for niche songs. Importing your own audio files is the most reliable way to add those songs.

---

## Features

**Library**
- Scans MP3, FLAC, OGG, Opus, M4A, AAC, WAV
- Fast full-text search across titles, artists, and albums
- Album art loaded from embedded tags or Cover Art Archive
- Watches your folders and updates automatically when files change

**Playback**
- Native audio engine — low latency, no web audio quirks
- Shuffle, repeat (track / queue), and volume control
- Lyrics view synced to the current track
- Full-screen player mode

**Discovery & Smart Mixes**
- Kaze tracks what you play and builds a taste profile over time
- Smart mixes: *Daily Mix*, *On Repeat*, *Forgotten Favorites*, *Late Night*, *Discovery*
- Listening stats with rolling windows: Today, 7 Days, 30 Days, All Time

**Playlists**
- Create and manage playlists
- Import playlists from Spotify (optional, needs your own Spotify API key)

**Wishlist & Downloads** *(optional)*
- Mark tracks you want to find
- Search available download sources, including direct streams, Internet Archive, and Audius
- Optionally connect a local [Slskd](https://github.com/slskd/slskd) daemon to search Soulseek and download into your library

---

## Download & Install

Pre-built binaries and packages for all supported operating systems are available on the [Releases page](https://github.com/selfAnnihilator/kaze/releases/latest). See **[INSTALL.md](INSTALL.md)** for detailed installation notes.

### Arch Linux / CachyOS / Manjaro / EndeavourOS

Kaze is published on the AUR as `kaze`. The native Arch package links against your system libraries for the best performance and is strongly preferred over the AppImage on Arch-based systems.

Install using `yay`:
```bash
yay -S kaze
```

Or using `paru`:
```bash
paru -S kaze
```

### Ubuntu / Debian / Linux Mint / Pop!_OS

Download the latest `.deb` from the [Releases page](https://github.com/selfAnnihilator/kaze/releases/latest), then install it:

```bash
sudo apt install ./kaze_*.deb
```

### Fedora / RHEL

Download the latest `.rpm` from the [Releases page](https://github.com/selfAnnihilator/kaze/releases/latest), then install it:

```bash
sudo dnf install ./kaze_*.rpm
```

### openSUSE

Download the latest `.rpm` from the [Releases page](https://github.com/selfAnnihilator/kaze/releases/latest), then install it:

```bash
sudo zypper install ./kaze_*.rpm
```

### Generic Linux (AppImage)

The AppImage is the generic, portable Linux option. It runs on any modern Linux distribution without installation.

Download the `.AppImage` from the [Releases page](https://github.com/selfAnnihilator/kaze/releases/latest), make it executable, and run:

```bash
chmod +x Kaze_*.AppImage
./Kaze_*.AppImage
```

### Windows

1. Download `Kaze_*_x64-setup.exe` from the [Releases page](https://github.com/selfAnnihilator/kaze/releases/latest) and run it.
2. Follow the setup wizard to complete installation.

*(An alternative `.msi` installer is also available on the Releases page.)*

---

## Automatic updates

For standalone installations (Windows and AppImage), Kaze automatically checks for updates on launch, downloads the latest version in the background, and restarts the app.

> **Note:** If you installed Kaze via the AUR (`yay` / `paru`) or a system package manager (`.deb` / `.rpm`), update the app through your package manager rather than relying on the built-in updater.

---

## First launch

1. Open Kaze.
2. Click **Add Folder** and select your music directory (e.g. `~/Music`).
3. Kaze scans and indexes your library — takes a few seconds for most collections.
4. Start listening.

---

## Optional setup

**Spotify metadata** (album art, richer metadata)
Go to Settings → Providers → enter your [Spotify Developer](https://developer.spotify.com/dashboard) Client ID and Secret. Kaze uses this only for metadata lookups — it never plays Spotify streams.

**Soulseek downloads**
Run [Slskd](https://github.com/slskd/slskd) locally, then go to Settings → Downloads → enter your Slskd URL and API key.

---

## Supported formats

FLAC · MP3 · OGG · Opus · M4A · AAC · WAV

---

## Built with

[Rust](https://www.rust-lang.org/) · [Tauri v2](https://tauri.app/) · [React 19](https://react.dev/) · [SQLite](https://www.sqlite.org/)

---

## License

MIT — free to use, modify, and distribute.
