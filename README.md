<p align="center">
  <img src="assets/icon.png" width="160" height="160" alt="PhoCast Logo" />
</p>

<h1 align="center">PhoCast</h1>

<p align="center">
  <strong>High-Performance, Zero-Latency Smartphone Webcam Bridge for PC, OBS Studio & NVIDIA Broadcast</strong>
</p>

<p align="center">
  <a href="https://github.com/otattt/PhoCast/releases/latest"><img src="https://img.shields.io/github/v/release/otattt/PhoCast?style=for-the-badge&color=00E676" alt="Latest Release" /></a>
  <a href="https://github.com/otattt/PhoCast/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge" alt="License" /></a>
  <img src="https://img.shields.io/badge/Platform-Android%20%7C%20Windows-lightgrey.svg?style=for-the-badge" alt="Platforms" />
</p>

---

## 📥 Direct Downloads (v1.0.0)

No extraction or unzipping needed! Download the raw files directly:

| Platform | File | Description | Download |
| :--- | :--- | :--- | :--- |
| 📱 **Android** | `PhoCast-v1.0.0.apk` | Phone camera streamer app (APK) | [**Download APK**](https://github.com/otattt/PhoCast/releases/download/v1.0.0/PhoCast-v1.0.0.apk) |
| 💻 **Windows** | `PhoCast-Setup-v1.0.0.exe` | **All-in-One Installer** (Client + OBS Plugin + Cleaner) | [**Download Installer**](https://github.com/otattt/PhoCast/releases/download/v1.0.0/PhoCast-Setup-v1.0.0.exe) |
| 🎥 **OBS Studio** | `phocast_obs.dll` | **Native OBS Studio Plugin** (Direct GPU pipeline) | [**Download Plugin**](https://github.com/otattt/PhoCast/releases/download/v1.0.0/phocast_obs.dll) |
| 📦 **Windows** | `PhoCast.exe` | Standalone portable PC client | [**Download Portable**](https://github.com/otattt/PhoCast/releases/download/v1.0.0/PhoCast.exe) |

---

## ✨ Features

- ⚡ **Zero-Latency Streaming:** Custom bulk memory transfer and real-time drop-frame policy over USB (ADB) or local Wi-Fi to eliminate buffer delay.
- 🎯 **Crisp 1080p Full HD:** CameraX 1.3 `ResolutionSelector` guarantees native high-resolution capture on Google Pixel and modern smartphones.
- 🧊 **Keep-Cool Mode (AMOLED Screen Off):** Turns the smartphone screen pitch-black and pauses local GPU preview rendering during streams, keeping the device cool to the touch and preserving battery life.
- 🎥 **Dedicated OBS Studio Plugin (`phocast_obs.dll`):** Feeds live frames directly into OBS Studio's internal graphics compositor. **100% crash-proof**, zero green screens, and zero virtual camera driver conflicts!
- 🌐 **Driverless HTTP MJPEG Server:** PhoCast PC client includes a built-in HTTP video stream on `http://127.0.0.1:8080/video`, compatible with OBS Browser / Media Source or VLC.
- 🎙️ **NVIDIA Broadcast & Discord Ready:** Stream to OBS Studio, then click **Start Virtual Camera** in OBS to broadcast studio video into NVIDIA Broadcast, Discord, Zoom, and Google Meet!
- 🖥️ **Native Windows Client:** Built with Rust and OpenGL GUI (`eframe/glow`)—ultra-lightweight (~15 MB RAM), instant startup, no CMD console windows.
- 🔒 **100% Local & Private:** Direct socket connection on your local machine and Wi-Fi—no cloud servers, no telemetry, no paid limits.

---

## 🚀 Setup & Usage Guide

### Step 1: Install PhoCast on Your Android Phone
1. Download [**`PhoCast-v1.0.0.apk`**](https://github.com/otattt/PhoCast/releases/download/v1.0.0/PhoCast-v1.0.0.apk) on your Android phone and install it.
2. Grant camera permissions.
3. Choose your preferred resolution (`1080p`, `720p`, `480p`) and frame rate (`30 FPS` or `60 FPS`).
4. Tap **START STREAM**.
5. *(Optional)* Tap **KEEP-COOL (SCREEN OFF)** to turn the screen pitch black and reduce heat on long streaming sessions.

---

### Step 2: Use PhoCast Directly in OBS Studio (Recommended)

1. **Install Plugin:**
   - Run [**`PhoCast-Setup-v1.0.0.exe`**](https://github.com/otattt/PhoCast/releases/download/v1.0.0/PhoCast-Setup-v1.0.0.exe) (it automatically detects and installs the plugin into OBS Studio).
   - *Or manually:* Copy [**`phocast_obs.dll`**](https://github.com/otattt/PhoCast/releases/download/v1.0.0/phocast_obs.dll) to:  
     `C:\Program Files\obs-studio\obs-plugins\64bit\`  
     (or run `install_obs_plugin.bat`).
2. **Open OBS Studio:**
   - In the **Sources** pane, click **`+`** (Add Source).
   - Select **PhoCast Camera**.
   - Set IP to `127.0.0.1` (for USB mode) or your phone's Wi-Fi IP address.
   - Click **OK**. Your phone camera stream is now live in OBS!

---

### Step 3: Stream to NVIDIA Broadcast, Discord, Zoom & Teams

NVIDIA Broadcast and conferencing apps require a hardware or WMF camera. The universal, studio-grade workflow is:

```
[Phone Stream] ──> [OBS Studio (PhoCast Source)] ──> [Start Virtual Camera] ──> [NVIDIA Broadcast / Discord / Zoom]
```

1. In OBS Studio with **PhoCast Camera** active, click **Start Virtual Camera** in the bottom-right Controls dock.
2. Open **NVIDIA Broadcast**, **Discord**, or **Zoom**.
3. Under Camera selection, choose **OBS Virtual Camera**.
4. Enjoy AI background blur, noise removal, eye-contact correction, and full 1080p streaming with maximum stability!

---

### Step 4: Clean Up Legacy DirectShow Drivers (If Installed Previously)

If you previously tested DirectShow virtual camera drivers that produced green screens or crashed OBS:
1. Double-click [**`clean_driver.bat`**](clean_driver.bat).
2. Click **Yes** when Windows prompts for Administrator privileges.
3. The script will unregister and cleanly wipe all DirectShow filter registry entries.

---

## 🛠️ Architecture

```
[ Android Smartphone ]
      │ (CameraX 1.3 1080p YUV_420_888)
      ▼
 [ Fast Bulk NV21 Converter ]  <-- ~2ms conversion time
      │ (JPEG 75% Quality - Thermal Optimized)
      ▼
[ TCP Streamer (Port 5000) ]  <-- Atomic drop-frame policy (Zero buffer bloat)
      │
      ├──> (USB ADB / Wi-Fi LAN)
      ▼
[ OBS Studio Native Plugin (phocast_obs.dll) ]  <-- Direct GPU frame ingestion
      │
      ▼
[ OBS Virtual Camera ] ──> [ NVIDIA Broadcast / Discord / Zoom / Teams ]
```

---

## 📄 License
Released under the [MIT License](LICENSE).
