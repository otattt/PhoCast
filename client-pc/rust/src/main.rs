#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::Read;
use std::net::TcpStream;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;
use image::GenericImageView;
use virtualcam::{BackendKind, Camera, PixelFormat};

#[derive(PartialEq, Clone, Copy)]
enum ConnectMode {
    Wifi,
    Usb,
}

struct VideoFrame {
    width: usize,
    height: usize,
    rgba_bytes: Vec<u8>,
}

struct PhoCastApp {
    mode: ConnectMode,
    ip_address: String,
    port: String,
    enable_vcam: bool,
    show_preview: bool,

    is_streaming: Arc<AtomicBool>,
    status_message: Arc<Mutex<String>>,
    current_fps: Arc<Mutex<f32>>,

    latest_frame: Arc<Mutex<Option<VideoFrame>>>,
    texture: Option<egui::TextureHandle>,
}

impl Default for PhoCastApp {
    fn default() -> Self {
        Self {
            mode: ConnectMode::Wifi,
            ip_address: "192.168.1.50".to_string(),
            port: "5000".to_string(),
            enable_vcam: true,
            show_preview: true,

            is_streaming: Arc::new(AtomicBool::new(false)),
            status_message: Arc::new(Mutex::new("Ready. Select connection mode and press Start.".to_string())),
            current_fps: Arc::new(Mutex::new(0.0)),

            latest_frame: Arc::new(Mutex::new(None)),
            texture: None,
        }
    }
}

impl PhoCastApp {
    fn start_stream(&mut self) {
        if self.is_streaming.load(Ordering::SeqCst) {
            return;
        }

        self.is_streaming.store(true, Ordering::SeqCst);
        let is_streaming = Arc::clone(&self.is_streaming);
        let status_message = Arc::clone(&self.status_message);
        let current_fps = Arc::clone(&self.current_fps);
        let latest_frame = Arc::clone(&self.latest_frame);

        let mode = self.mode;
        let mut target_ip = self.ip_address.trim().to_string();
        let port_u16 = self.port.trim().parse::<u16>().unwrap_or(5000);
        let enable_vcam = self.enable_vcam;

        thread::spawn(move || {
            if mode == ConnectMode::Usb {
                *status_message.lock().unwrap() = "Setting up ADB port forward...".to_string();
                target_ip = "127.0.0.1".to_string();

                let adb_cmd = if Command::new("adb").arg("version").output().is_ok() {
                    "adb".to_string()
                } else {
                    let appdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
                    let fallback = format!("{}\\Android\\Sdk\\platform-tools\\adb.exe", appdata);
                    if std::path::Path::new(&fallback).exists() {
                        fallback
                    } else {
                        "adb".to_string()
                    }
                };

                let forward_res = Command::new(&adb_cmd)
                    .args(["forward", &format!("tcp:{}", port_u16), &format!("tcp:{}", port_u16)])
                    .output();

                if forward_res.map(|o| o.status.success()).unwrap_or(false) {
                    *status_message.lock().unwrap() = "ADB forwarded. Connecting to localhost...".to_string();
                } else {
                    *status_message.lock().unwrap() = "ADB forward failed. Is phone connected via USB?".to_string();
                }
            }

            let target_addr = format!("{}:{}", target_ip, port_u16);
            *status_message.lock().unwrap() = format!("Connecting to {}...", target_addr);

            let mut stream = match TcpStream::connect(&target_addr) {
                Ok(s) => {
                    s.set_nodelay(true).ok();
                    s.set_read_timeout(Some(Duration::from_secs(5))).ok();
                    *status_message.lock().unwrap() = format!("Connected to {}", target_addr);
                    s
                }
                Err(e) => {
                    *status_message.lock().unwrap() = format!("Connection error: {}", e);
                    is_streaming.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let mut vcam: Option<Camera> = None;
            let mut vcam_width = 1280u32;
            let mut vcam_height = 720u32;

            if enable_vcam {
                if let Ok(cam) = Camera::builder(vcam_width, vcam_height, 30.0)
                    .format(PixelFormat::BGR)
                    .backend(BackendKind::Obs)
                    .build()
                {
                    vcam = Some(cam);
                }
            }

            let mut len_buf = [0u8; 4];
            let mut frame_count = 0u32;
            let mut last_fps_time = Instant::now();

            while is_streaming.load(Ordering::SeqCst) {
                if stream.read_exact(&mut len_buf).is_err() {
                    break;
                }

                let payload_len = u32::from_be_bytes(len_buf) as usize;
                if payload_len == 0 || payload_len > 10 * 1024 * 1024 {
                    continue;
                }

                let mut img_bytes = vec![0u8; payload_len];
                if stream.read_exact(&mut img_bytes).is_err() {
                    break;
                }

                if let Ok(img) = image::load_from_memory(&img_bytes) {
                    let (w, h) = img.dimensions();
                    let (w, h) = (w as usize, h as usize);

                    let rgba = img.to_rgba8();
                    let rgba_raw = rgba.into_raw();

                    // Feed to OBS Virtual Camera
                    if let Some(ref mut cam) = vcam {
                        if w as u32 != vcam_width || h as u32 != vcam_height {
                            vcam_width = w as u32;
                            vcam_height = h as u32;
                        }

                        // Convert RGBA to BGR
                        let mut bgr_buf = Vec::with_capacity(w * h * 3);
                        for chunk in rgba_raw.chunks_exact(4) {
                            bgr_buf.push(chunk[2]); // B
                            bgr_buf.push(chunk[1]); // G
                            bgr_buf.push(chunk[0]); // R
                        }
                        let _ = cam.send(&bgr_buf);
                    }

                    // Update UI preview frame
                    *latest_frame.lock().unwrap() = Some(VideoFrame {
                        width: w,
                        height: h,
                        rgba_bytes: rgba_raw,
                    });

                    frame_count += 1;
                    if last_fps_time.elapsed().as_secs() >= 1 {
                        let fps = frame_count as f32 / last_fps_time.elapsed().as_secs_f32();
                        *current_fps.lock().unwrap() = fps;
                        frame_count = 0;
                        last_fps_time = Instant::now();
                    }
                }
            }

            is_streaming.store(false, Ordering::SeqCst);
            *status_message.lock().unwrap() = "Disconnected / Idle".to_string();
        });
    }

    fn stop_stream(&mut self) {
        self.is_streaming.store(false, Ordering::SeqCst);
        *self.status_message.lock().unwrap() = "Disconnected".to_string();
    }
}

impl eframe::App for PhoCastApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request continuous repaint while streaming for smooth video preview
        if self.is_streaming.load(Ordering::SeqCst) {
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Header
            ui.horizontal(|ui| {
                ui.heading("📸 PhoCast Client");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let is_active = self.is_streaming.load(Ordering::SeqCst);
                    if is_active {
                        let fps = *self.current_fps.lock().unwrap();
                        ui.colored_label(egui::Color32::from_rgb(0, 230, 118), format!("● LIVE ({:.1} FPS)", fps));
                    } else {
                        ui.colored_label(egui::Color32::GRAY, "○ IDLE");
                    }
                });
            });

            ui.separator();

            // Connection Mode Tabs (DroidCam Style)
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.mode, ConnectMode::Wifi, "📶 Wi-Fi (LAN)");
                ui.selectable_value(&mut self.mode, ConnectMode::Usb, "🔌 USB Cable (ADB)");
            });

            ui.add_space(8.0);

            // Input Fields
            egui::Grid::new("connect_grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                if self.mode == ConnectMode::Wifi {
                    ui.label("Device IP:");
                    ui.text_edit_singleline(&mut self.ip_address);
                    ui.end_row();
                }

                ui.label("Port:");
                ui.text_edit_singleline(&mut self.port);
                ui.end_row();
            });

            ui.add_space(8.0);

            // Checkbox Options
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.enable_vcam, "Virtual Camera (OBS)");
                ui.checkbox(&mut self.show_preview, "Show Preview");
            });

            ui.add_space(10.0);

            // Start / Stop Button
            let is_active = self.is_streaming.load(Ordering::SeqCst);
            let btn_text = if is_active { "⏹  STOP" } else { "▶  START" };
            let btn_color = if is_active {
                egui::Color32::from_rgb(220, 20, 60) // Red
            } else {
                egui::Color32::from_rgb(0, 180, 80) // Green
            };

            let btn = egui::Button::new(egui::RichText::new(btn_text).size(16.0).color(egui::Color32::WHITE))
                .fill(btn_color)
                .min_size(egui::vec2(ui.available_width(), 38.0));

            if ui.add(btn).clicked() {
                if is_active {
                    self.stop_stream();
                } else {
                    self.start_stream();
                }
            }

            ui.add_space(6.0);

            // Status message bar
            let status = self.status_message.lock().unwrap().clone();
            ui.label(egui::RichText::new(format!("Status: {}", status)).small().color(egui::Color32::LIGHT_GRAY));

            ui.add_space(6.0);

            // Video Preview Area
            if self.show_preview {
                ui.separator();
                let frame_opt = self.latest_frame.lock().unwrap().take();
                if let Some(frame) = frame_opt {
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        [frame.width, frame.height],
                        &frame.rgba_bytes,
                    );
                    self.texture = Some(ctx.load_texture("camera_preview", color_image, Default::default()));
                }

                if let Some(ref texture) = self.texture {
                    let avail_width = ui.available_width();
                    let aspect = texture.size()[0] as f32 / texture.size()[1] as f32;
                    let display_size = egui::vec2(avail_width, avail_width / aspect);
                    ui.image((texture.id(), display_size));
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(30.0);
                        ui.label(egui::RichText::new("No active camera preview\nPress START to connect").weak());
                        ui.add_space(30.0);
                    });
                }
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([440.0, 560.0])
            .with_min_inner_size([380.0, 480.0])
            .with_title("PhoCast Client"),
        ..Default::default()
    };

    eframe::run_native(
        "PhoCast Client",
        native_options,
        Box::new(|_cc| Ok(Box::new(PhoCastApp::default()))),
    )
}
