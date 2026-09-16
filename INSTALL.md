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

### Option 2: Arch Linux / CachyOS / Manjaro (System & Terminal Integration)

To make `kaze` executable from any terminal and register the desktop entry in your application launcher:

```bash
# 1. Download Kaze_*.AppImage and make executable
chmod +x Kaze_*.AppImage

# 2. Install to your local user binary directory
mkdir -p ~/.local/bin
cp Kaze_*.AppImage ~/.local/bin/kaze

# 3. Extract desktop entry and icon for system app launcher
mkdir -p ~/.local/share/applications ~/.local/share/icons/hicolor/512x512/apps
~/.local/bin/kaze --appimage-extract "usr/share/applications/Kaze.desktop"
~/.local/bin/kaze --appimage-extract "usr/share/icons/hicolor/512x512/apps/kaze.png"
cp squashfs-root/usr/share/applications/Kaze.desktop ~/.local/share/applications/kaze.desktop
cp squashfs-root/usr/share/icons/hicolor/512x512/apps/kaze.png ~/.local/share/icons/hicolor/512x512/apps/kaze.png
sed -i "s|^Exec=.*|Exec=$HOME/.local/bin/kaze|" ~/.local/share/applications/kaze.desktop
rm -rf squashfs-root
update-desktop-database ~/.local/share/applications 2>/dev/null || true
```

After running this once:
- Type `kaze` in your terminal to start the player anytime.
- Search "Kaze" in your app menu (Rofi, Wofi, GNOME, KDE) to launch it.

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
