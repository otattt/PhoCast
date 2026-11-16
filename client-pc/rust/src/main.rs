use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::Command;
use std::time::{Duration, Instant};

use image::GenericImageView;
use minifb::{Key, Window, WindowOptions};
use virtualcam::{BackendKind, Camera, PixelFormat};

fn setup_adb_forward(port: u16) -> bool {
    println!("[*] Setting up ADB port forward: tcp:{} -> tcp:{}...", port, port);

    // Try finding adb in PATH or default Android Sdk path
    let adb_cmd = if Command::new("adb").arg("version").output().is_ok() {
        "adb".to_string()
    } else {
        let appdata = env::var("LOCALAPPDATA").unwrap_or_default();
        let fallback = format!("{}\\Android\\Sdk\\platform-tools\\adb.exe", appdata);
        if std::path::Path::new(&fallback).exists() {
            fallback
        } else {
            "adb".to_string()
        }
    };

    let check = Command::new(&adb_cmd).arg("devices").output();
    match check {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let has_device = stdout.lines().skip(1).any(|l| l.contains("\tdevice"));
            if !has_device {
                println!("[!] No Android device detected via ADB.");
                println!("    Please ensure USB Debugging is enabled on your phone.");
                return false;
            }

            let forward = Command::new(&adb_cmd)
                .args(["forward", &format!("tcp:{}", port), &format!("tcp:{}", port)])
                .output();

            if forward.map(|o| o.status.success()).unwrap_or(false) {
                println!("[✓] ADB forward successful.");
                true
            } else {
                println!("[!] ADB forward command failed.");
                false
            }
        }
        Err(e) => {
            println!("[!] Failed to invoke ADB: {}. Please check Android Platform Tools.", e);
            false
        }
    }
}

fn main() {
    println!("============================================================");
    println!("         PhoCast Native Desktop Client (Rust Edition)        ");
    println!("============================================================");

    let args: Vec<String> = env::args().collect();
    let mut use_usb = false;
    let mut host = "127.0.0.1".to_string();
    let mut port: u16 = 5000;
    let mut enable_vcam = true;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--usb" => use_usb = true,
            "--ip" => {
                if i + 1 < args.len() {
                    host = args[i + 1].clone();
                    i += 1;
                }
            }
            "--port" => {
                if i + 1 < args.len() {
                    if let Ok(p) = args[i + 1].parse::<u16>() {
                        port = p;
                    }
                    i += 1;
                }
            }
            "--no-vcam" => enable_vcam = false,
            "--help" | "-h" => {
                println!("Usage: phocast [OPTIONS]");
                println!("Options:");
                println!("  --usb        Connect via USB cable (runs adb forward automatically)");
                println!("  --ip <IP>    Target phone IP address for Wi-Fi mode (default: 127.0.0.1)");
                println!("  --port <P>   Stream port (default: 5000)");
                println!("  --no-vcam    Disable virtual camera output, show preview window only");
                return;
            }
            _ => {}
        }
        i += 1;
    }

    if use_usb {
        host = "127.0.0.1".to_string();
        if !setup_adb_forward(port) {
            println!("[!] Continuing anyway in case port forwarding was set manually...");
        }
    }

    let target_addr = format!("{}:{}", host, port);
    println!("[*] Connecting to PhoCast stream at {}...", target_addr);

    let mut stream = match TcpStream::connect(&target_addr) {
        Ok(s) => {
            s.set_nodelay(true).ok();
            s.set_read_timeout(Some(Duration::from_secs(5))).ok();
            println!("[✓] Connected to PhoCast stream!");
            s
        }
        Err(e) => {
            eprintln!("[!] Connection failed: {}", e);
            eprintln!("    Make sure PhoCast is running on your phone and 'START STREAM' is active.");
            return;
        }
    };

    let mut win_width = 1280;
    let mut win_height = 720;

    let mut window = match Window::new(
        "PhoCast Native Live Preview - [Q / ESC to exit]",
        win_width,
        win_height,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    ) {
        Ok(win) => win,
        Err(e) => {
            eprintln!("[!] Failed to create preview window: {}", e);
            return;
        }
    };

    // Limit window update rate
    window.limit_update_rate(Some(Duration::from_micros(16600))); // ~60 FPS

    let mut vcam: Option<Camera> = None;
    if enable_vcam {
        match Camera::builder(win_width as u32, win_height as u32, 30.0)
            .format(PixelFormat::BGR)
            .backend(BackendKind::Obs)
            .build()
        {
            Ok(cam) => {
                println!("[✓] Windows Virtual Camera active via OBS driver!");
                println!("    Select 'OBS Virtual Camera' in Zoom, Google Meet, Teams, Discord, etc.");
                vcam = Some(cam);
            }
            Err(e) => {
                println!("[!] Virtual camera not initialized ({:?}). Showing window preview only.", e);
            }
        }
    }

    let mut buffer: Vec<u32> = vec![0; win_width * win_height];
    let mut vcam_frame_bgr: Vec<u8> = vec![0; win_width * win_height * 3];

    let mut len_buf = [0u8; 4];
    let mut frame_count = 0u32;
    let mut last_fps_time = Instant::now();

    println!("[*] Video streaming active. Press ESC or Q to exit.");

    while window.is_open() && !window.is_key_down(Key::Escape) && !window.is_key_down(Key::Q) {
        // Read 4-byte big-endian payload length
        if stream.read_exact(&mut len_buf).is_err() {
            println!("[!] Stream ended or timeout.");
            break;
        }

        let payload_len = u32::from_be_bytes(len_buf) as usize;
        if payload_len == 0 || payload_len > 10 * 1024 * 1024 {
            // Out of bounds packet protection
            continue;
        }

        let mut img_bytes = vec![0u8; payload_len];
        if stream.read_exact(&mut img_bytes).is_err() {
            println!("[!] Incomplete frame received.");
            break;
        }

        // Decode frame
        if let Ok(img) = image::load_from_memory(&img_bytes) {
            let (w, h) = img.dimensions();
            let (w, h) = (w as usize, h as usize);

            if w != win_width || h != win_height {
                win_width = w;
                win_height = h;
                buffer.resize(win_width * win_height, 0);
                vcam_frame_bgr.resize(win_width * win_height * 3, 0);
            }

            let rgb_img = img.to_rgb8();
            let raw_bytes = rgb_img.as_raw();

            // Populate preview buffer (u32 0x00RRGGBB) and vcam buffer (BGR)
            for (idx, chunk) in raw_bytes.chunks_exact(3).enumerate() {
                let r = chunk[0];
                let g = chunk[1];
                let b = chunk[2];

                if idx < buffer.len() {
                    buffer[idx] = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                }

                let bgr_idx = idx * 3;
                if bgr_idx + 2 < vcam_frame_bgr.len() {
                    vcam_frame_bgr[bgr_idx] = b;
                    vcam_frame_bgr[bgr_idx + 1] = g;
                    vcam_frame_bgr[bgr_idx + 2] = r;
                }
            }

            // Send to Virtual Camera
            if let Some(ref mut cam) = vcam {
                let _ = cam.send(&vcam_frame_bgr);
            }

            // Render to local window
            window.update_with_buffer(&buffer, win_width, win_height).ok();
            frame_count += 1;

            if last_fps_time.elapsed().as_secs() >= 5 {
                let fps = frame_count as f64 / last_fps_time.elapsed().as_secs_f64();
                window.set_title(&format!("PhoCast Native Client - {:.1} FPS ({}x{})", fps, win_width, win_height));
                frame_count = 0;
                last_fps_time = Instant::now();
            }
        }
    }

    println!("[✓] PhoCast client stopped gracefully.");
}
