# PhoCast 📸⚡

> **High-Performance, Low-Latency Phone-to-PC Webcam & Video Streaming Bridge**

PhoCast transforms your Android smartphone into a high-definition webcam for your PC (Windows/Linux) with ultra-low latency, hardware-accelerated H.264 encoding, and seamless connection via USB (ADB) or Wi-Fi.

---

## 🌟 Key Features

- **Zero-Copy Hardware Encoding:** Captures directly from CameraX via hardware OpenGL Surface to `MediaCodec` (H.264 / AVC) with near-zero CPU overhead.
- **Ultra-Low Latency:** High-speed TCP stream with `TCP_NODELAY` and lightweight NAL unit framing.
- **Dual Connection Modes:**
  - **USB Mode:** Automatic port-forwarding via ADB (`adb forward tcp:5000 tcp:5000`) for rock-solid stability and zero Wi-Fi congestion.
  - **Wi-Fi Mode:** Direct LAN socket streaming with automatic IP discovery.
- **Virtual Camera Integration:** Appears as a real hardware camera in Zoom, Google Meet, Microsoft Teams, Discord, and OBS Studio.
- **Privacy First:** 100% local streaming on your own machine and network—no cloud, no accounts, no tracking.

---

## 📂 Repository Structure

```
PhoCast/
├── android/             # Android Streamer App (Kotlin, CameraX, MediaCodec)
│   ├── app/
│   │   ├── src/main/java/com/phocast/streamer/
│   │   │   ├── camera/      # CameraX manager & H.264 Hardware Encoder
│   │   │   ├── network/     # TCP Server & Network discovery
│   │   │   └── ui/          # Modern UI with resolution & bitrate settings
│   │   └── build.gradle.kts
│   └── settings.gradle.kts
│
├── client-pc/           # Windows & Cross-Platform Desktop Client
│   ├── python/          # Lightweight Python client with Virtual Camera driver bridge
│   └── rust/            # High-performance standalone native client
│
└── docs/                # Architecture diagrams & protocol specifications
    └── ARCHITECTURE.md
```

---

## 🚀 Quick Start

### 1. Android App
1. Open the `android/` directory in **Android Studio**.
2. Connect your Android phone (Android 7.0 / API 24+) with **Developer Options > USB Debugging** enabled.
3. Build and run `app` on your phone.
4. Press **"Start Streaming"**. The app will show its local IP and port (default: `5000`).

### 2. PC Client (Windows)
#### Using Python Client:
```bash
cd client-pc/python
pip install -r requirements.txt
python phocast_client.py --usb
```
Or for Wi-Fi:
```bash
python phocast_client.py --ip 192.168.1.50
```

---

## 📜 Protocol & Architecture
See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed technical specifications on:
- Hardware encoder pipeline (CameraX Surface -> MediaCodec -> TCP Socket)
- H.264 Annex-B packetization and SPS/PPS transmission
- Windows Media Foundation & OBS Virtual Camera bridges

---

## 📄 License
Released under the [MIT License](LICENSE).
