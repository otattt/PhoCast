# PhoCast Architecture & Protocol Specification

## 1. Overview

PhoCast replaces slow, CPU-heavy webcam solutions (such as continuous JPEG screenshot loops) with a modern **Hardware-Accelerated Zero-Copy Streaming Pipeline**.

```
┌─────────────────────────────────────────────────────────────┐
│                   ANDROID (Camera Source)                   │
│                                                             │
│  [ Camera Sensor ]                                          │
│         │                                                   │
│         ▼ (Camera2 / CameraX)                               │
│  [ Camera SurfaceProvider ]                                 │
│         │ (EGL Hardware Surface - Zero CPU copy)            │
│         ▼                                                   │
│  [ MediaCodec H.264 Hardware Encoder ]                      │
│         │                                                   │
│         ▼ (NAL Units: SPS, PPS, IDR, Non-IDR)               │
│  [ TCP Server Socket (Port 5000) ]                          │
└─────────┬───────────────────────────────────────────────────┘
          │
          │ TCP / USB Tunnel (adb forward)
          │
┌─────────▼───────────────────────────────────────────────────┐
│                    PC CLIENT (Windows)                      │
│                                                             │
│  [ TCP Connection Manager ]                                 │
│         │                                                   │
│         ▼ (Annex-B Byte Stream)                             │
│  [ H.264 Stream Parser & Decoder ]                          │
│         │                                                   │
│         ▼ (Raw BGRA Frame Buffer)                           │
│  [ Virtual Camera Sink ]                                    │
│    ├── OBS VirtualCam Shared Memory                         │
│    └── Windows Media Foundation Virtual Camera API          │
│         │                                                   │
│         ▼                                                   │
│  [ Applications: Zoom, Meet, Teams, Discord, OBS ]          │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Stream Format & Protocol

### Transport
- **Protocol:** Raw TCP Socket
- **Default Port:** `5000`
- **Socket Options:** `TCP_NODELAY = true` (disables Nagle's algorithm for sub-10ms packet dispatch)

### Payload Format (H.264 Annex-B)
Video frames are streamed as standard **H.264 NAL units in Annex-B format**:
Each NAL unit begins with a 4-byte start code:
```
[ 0x00, 0x00, 0x00, 0x01 ] + [ NAL Header ] + [ NAL Payload ]
```

1. **SPS (Sequence Parameter Set) & PPS (Picture Parameter Set):**
   - Transmitted when connection is established and with every Keyframe (IDR).
   - Contains video resolution, aspect ratio, profile, and level.
2. **IDR (Instantaneous Decoder Refresh - Keyframe):**
   - Encoded every 1-2 seconds (`KEY_I_FRAME_INTERVAL = 1`) to ensure instant client reconnection and frame recovery if network packets are dropped.
3. **Non-IDR Slice (P-Frames):**
   - Delta updates sent at 30/60 FPS for minimal bandwidth.

---

## 3. Dual Connection Modes

### USB Mode (Zero Congestion & Ultra Stable)
```bash
# 1. PC Client queries connected devices
adb devices

# 2. Port forward host port 5000 to phone port 5000
adb forward tcp:5000 tcp:5000

# 3. PC connects to localhost
Connect to 127.0.0.1:5000
```

### Wi-Fi Mode (Wireless Convenience)
- Phone displays local IP (e.g. `192.168.1.50`).
- PC client connects directly to `192.168.1.50:5000`.
- Supports mDNS broadcast for zero-configuration discovery.
