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

See **[INSTALL.md](INSTALL.md)** for full per-platform instructions.

| Platform | File to download |
|---|---|
| 🐧 Linux (Arch / CachyOS / Manjaro) | `.AppImage` (install via terminal script below) |
| 🐧 Linux (Ubuntu / Debian / Mint) | `.deb` |
| 🐧 Linux (Fedora / openSUSE / RHEL) | `.rpm` |
| 🐧 Linux (Generic / Portable) | `.AppImage` |
| 🪟 Windows | `_x64-setup.exe` |

All downloads are on the [Releases page](https://github.com/selfAnnihilator/kaze/releases/latest).

### Arch Linux / CachyOS quick setup
Download the `.AppImage` and integrate it as a native desktop application with `kaze` terminal command:

```bash
chmod +x Kaze_*.AppImage
mkdir -p ~/.local/bin
cp Kaze_*.AppImage ~/.local/bin/kaze

# Extract desktop entry & icon so it appears in application launchers
~/.local/bin/kaze --appimage-extract "*.desktop"
~/.local/bin/kaze --appimage-extract "*.png"
mkdir -p ~/.local/share/applications ~/.local/share/icons/hicolor/512x512/apps
cp squashfs-root/*.desktop ~/.local/share/applications/kaze.desktop 2>/dev/null || true
sed -i 's|^Exec=.*|Exec='$HOME'/.local/bin/kaze|' ~/.local/share/applications/kaze.desktop 2>/dev/null || true
cp squashfs-root/usr/share/icons/hicolor/512x512/apps/*.png ~/.local/share/icons/hicolor/512x512/apps/kaze.png 2>/dev/null || true
rm -rf squashfs-root
```
Now typing `kaze` in any terminal runs it, and it will show up when searching your desktop apps.

---

## Automatic updates

Once installed, Kaze checks for updates every time you open it. When a new version is ready it downloads in the background and restarts the app automatically. You don't need to do anything.

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
