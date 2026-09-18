# Installing Kaze

Kaze is a free, open-source desktop music player. Choose the installer for your operating system below.

---

## 🐧 Linux

### Option 1: AppImage (works on any Linux distro — recommended if unsure)

1. Download the `.AppImage` file from the [latest release](../../releases/latest).
2. Make it executable:
   ```bash
   chmod +x Kaze_*.AppImage
   ```
3. Double-click it, or run it from the terminal:
   ```bash
   ./Kaze_*.AppImage
   ```

> No installation needed. The AppImage is self-contained and runs on any modern Linux distribution.

---

### Option 2: Arch Linux / CachyOS / Manjaro (`.pkg.tar.zst` — Native Package)

Native Arch package built specifically for Arch Linux, CachyOS, and Manjaro using system WebKitGTK and GTK3 libraries for optimal performance.

1. Download `kaze-*.pkg.tar.zst` from the [latest release](../../releases/latest).
2. Install it with pacman:
   ```bash
   sudo pacman -U ./kaze-*.pkg.tar.zst
   ```
3. Launch Kaze from your app launcher (Rofi, Wofi, GNOME, KDE) or run `kaze` in any terminal.

> **Building from source / AUR:**
> You can also build and install the package locally from source using the included `PKGBUILD`:
> ```bash
> cd packaging/arch
> makepkg -si
> ```


---

### Option 3: Debian/Ubuntu (`.deb` — for Ubuntu, Mint, Pop!_OS, Debian)

1. Download the `.deb` file from the [latest release](../../releases/latest).
2. Install it:
   ```bash
   sudo dpkg -i kaze_*.deb
   # Or double-click the .deb in your file manager
   ```
3. Launch Kaze from your app menu.

---

### Option 4: Fedora / openSUSE / RHEL (`.rpm`)

1. Download the `.rpm` file from the [latest release](../../releases/latest).
2. Install it:
   ```bash
   sudo rpm -i kaze_*.rpm
   # Or with dnf:
   sudo dnf install kaze_*.rpm
   ```

---

## 🪟 Windows

### Option 1: NSIS Installer (`.exe` — recommended)

1. Download `Kaze_*_x64-setup.exe` from the [latest release](../../releases/latest).
2. Double-click the file.
3. Follow the installation wizard.
4. Launch Kaze from the Start Menu.

> **Windows SmartScreen warning:** If you see "Windows protected your PC", click **"More info"** then **"Run anyway"**. This happens because the app is not yet code-signed. It is safe.

### Option 2: MSI Installer (`.msi`)

1. Download `Kaze_*_x64_en-US.msi` from the [latest release](../../releases/latest).
2. Double-click to install.

---

## 🎵 First Launch

1. Open Kaze.
2. Click **"Add Music Folder"** and select the folder where your music lives (e.g. `~/Music` or `C:\Users\You\Music`).
3. Kaze will scan and index your library automatically.
4. Start listening!

---

## ❓ Troubleshooting

**Linux: App won't open / "permission denied"**
```bash
chmod +x Kaze_*.AppImage
```

**Linux: Missing audio (no sound)**
```bash
# Ubuntu/Debian
sudo apt install libasound2

# Fedora
sudo dnf install alsa-lib
```

**Windows: Missing WebView2 runtime**
The installer includes a WebView2 bootstrapper that downloads it automatically. If it fails, install it manually from [Microsoft's website](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).

---

*Need help? Open an issue on [GitHub](../../issues).*
