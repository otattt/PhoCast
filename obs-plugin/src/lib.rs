use std::ffi::{c_char, c_void, CStr, CString};
use std::io::Read;
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;

// ============================================================================
// OBS Studio Data Structures & Constants
// ============================================================================

pub const MAX_AV_PLANES: usize = 8;
pub const VIDEO_FORMAT_RGBA: i32 = 6;
pub const OBS_SOURCE_TYPE_INPUT: u32 = 0;
pub const OBS_SOURCE_VIDEO: u32 = 1 << 0;
pub const OBS_SOURCE_ASYNC: u32 = 1 << 2;
pub const OBS_SOURCE_ASYNC_VIDEO: u32 = OBS_SOURCE_ASYNC | OBS_SOURCE_VIDEO;
pub const OBS_ICON_TYPE_CAMERA: u32 = 9;
pub const OBS_TEXT_DEFAULT: i32 = 0;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ObsSourceFrame {
    pub data: [*mut u8; MAX_AV_PLANES],
    pub linesize: [u32; MAX_AV_PLANES],
    pub width: u32,
    pub height: u32,
    pub timestamp: u64,
    pub format: i32,
    pub color_matrix: [f32; 16],
    pub full_range: bool,
    pub max_luminance: u16,
    pub color_range_min: [f32; 3],
    pub color_range_max: [f32; 3],
    pub flip: bool,
    pub flags: u8,
    pub trc: u8,
    pub refs: std::ffi::c_long,
    pub prev_frame: bool,
}

impl Default for ObsSourceFrame {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
pub struct ObsSourceInfo {
    pub id: *const c_char,
    pub r#type: u32,
    pub output_flags: u32,
    pub get_name: Option<unsafe extern "C" fn(type_data: *mut c_void) -> *const c_char>,
    pub create: Option<unsafe extern "C" fn(settings: *mut c_void, source: *mut c_void) -> *mut c_void>,
    pub destroy: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub get_width: Option<unsafe extern "C" fn(data: *mut c_void) -> u32>,
    pub get_height: Option<unsafe extern "C" fn(data: *mut c_void) -> u32>,
    pub get_defaults: Option<unsafe extern "C" fn(settings: *mut c_void)>,
    pub get_properties: Option<unsafe extern "C" fn(data: *mut c_void) -> *mut c_void>,
    pub update: Option<unsafe extern "C" fn(data: *mut c_void, settings: *mut c_void)>,
    pub activate: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub deactivate: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub show: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub hide: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub video_tick: Option<unsafe extern "C" fn(data: *mut c_void, seconds: f32)>,
    pub video_render: Option<unsafe extern "C" fn(data: *mut c_void, effect: *mut c_void)>,
    pub filter_video: *mut c_void,
    pub filter_audio: *mut c_void,
    pub enum_active_sources: *mut c_void,
    pub save: Option<unsafe extern "C" fn(data: *mut c_void, settings: *mut c_void)>,
    pub load: Option<unsafe extern "C" fn(data: *mut c_void, settings: *mut c_void)>,
    pub mouse_click: *mut c_void,
    pub mouse_move: *mut c_void,
    pub mouse_wheel: *mut c_void,
    pub focus: *mut c_void,
    pub key_click: *mut c_void,
    pub filter_remove: *mut c_void,
    pub type_data: *mut c_void,
    pub free_type_data: *mut c_void,
    pub audio_render: *mut c_void,
    pub enum_all_sources: *mut c_void,
    pub transition_start: *mut c_void,
    pub transition_stop: *mut c_void,
    pub get_defaults2: *mut c_void,
    pub get_properties2: *mut c_void,
    pub audio_mix: *mut c_void,
    pub icon_type: u32,
    pub media_play_pause: *mut c_void,
    pub media_restart: *mut c_void,
    pub media_stop: *mut c_void,
    pub media_next: *mut c_void,
    pub media_previous: *mut c_void,
    pub media_get_duration: *mut c_void,
    pub media_get_time: *mut c_void,
    pub media_set_time: *mut c_void,
    pub media_get_state: *mut c_void,
    pub version: u32,
    pub unversioned_id: *const c_char,
    pub missing_files: *mut c_void,
    pub video_get_color_space: *mut c_void,
    pub filter_add: *mut c_void,
    pub get_dark_icon: *mut c_void,
    pub get_light_icon: *mut c_void,
}

unsafe impl Sync for ObsSourceInfo {}
unsafe impl Send for ObsSourceInfo {}

// ============================================================================
// Dynamic LibOBS API Resolution
// ============================================================================

struct LibObsApi {
    pub obs_get_version: Option<unsafe extern "C" fn() -> u32>,
    pub obs_register_source_s: Option<unsafe extern "C" fn(info: *const ObsSourceInfo, size: usize)>,
    pub obs_source_output_video: Option<unsafe extern "C" fn(source: *mut c_void, frame: *const ObsSourceFrame)>,
    pub os_gettime_ns: Option<unsafe extern "C" fn() -> u64>,
    pub obs_properties_create: Option<unsafe extern "C" fn() -> *mut c_void>,
    pub obs_properties_add_text: Option<unsafe extern "C" fn(props: *mut c_void, name: *const c_char, desc: *const c_char, text_type: i32) -> *mut c_void>,
    pub obs_properties_add_int: Option<unsafe extern "C" fn(props: *mut c_void, name: *const c_char, desc: *const c_char, min: i32, max: i32, step: i32) -> *mut c_void>,
    pub obs_data_get_string: Option<unsafe extern "C" fn(data: *mut c_void, name: *const c_char) -> *const c_char>,
    pub obs_data_get_int: Option<unsafe extern "C" fn(data: *mut c_void, name: *const c_char) -> i64>,
    pub obs_data_set_default_string: Option<unsafe extern "C" fn(data: *mut c_void, name: *const c_char, val: *const c_char)>,
    pub obs_data_set_default_int: Option<unsafe extern "C" fn(data: *mut c_void, name: *const c_char, val: i64)>,
}

static mut OBS_API: LibObsApi = LibObsApi {
    obs_get_version: None,
    obs_register_source_s: None,
    obs_source_output_video: None,
    os_gettime_ns: None,
    obs_properties_create: None,
    obs_properties_add_text: None,
    obs_properties_add_int: None,
    obs_data_get_string: None,
    obs_data_get_int: None,
    obs_data_set_default_string: None,
    obs_data_set_default_int: None,
};

static mut OBS_MODULE: *mut c_void = std::ptr::null_mut();

unsafe fn init_obs_api() {
    let obs_dll_name = b"obs.dll\0";
    let handle = GetModuleHandleA(obs_dll_name.as_ptr());
    if handle.is_null() {
        return;
    }

    macro_rules! resolve {
        ($field:ident, $sym:expr, $t:ty) => {
            if let Some(proc) = GetProcAddress(handle, concat!($sym, "\0").as_ptr()) {
                OBS_API.$field = Some(std::mem::transmute::<_, $t>(proc));
            }
        };
    }

    resolve!(obs_get_version, "obs_get_version", unsafe extern "C" fn() -> u32);
    resolve!(obs_register_source_s, "obs_register_source_s", unsafe extern "C" fn(*const ObsSourceInfo, usize));
    resolve!(obs_source_output_video, "obs_source_output_video", unsafe extern "C" fn(*mut c_void, *const ObsSourceFrame));
    resolve!(os_gettime_ns, "os_gettime_ns", unsafe extern "C" fn() -> u64);
    resolve!(obs_properties_create, "obs_properties_create", unsafe extern "C" fn() -> *mut c_void);
    resolve!(obs_properties_add_text, "obs_properties_add_text", unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char, i32) -> *mut c_void);
    resolve!(obs_properties_add_int, "obs_properties_add_int", unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char, i32, i32, i32) -> *mut c_void);
    resolve!(obs_data_get_string, "obs_data_get_string", unsafe extern "C" fn(*mut c_void, *const c_char) -> *const c_char);
    resolve!(obs_data_get_int, "obs_data_get_int", unsafe extern "C" fn(*mut c_void, *const c_char) -> i64);
    resolve!(obs_data_set_default_string, "obs_data_set_default_string", unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char));
    resolve!(obs_data_set_default_int, "obs_data_set_default_int", unsafe extern "C" fn(*mut c_void, *const c_char, i64));
}

// ============================================================================
// PhoCast Source Implementation
// ============================================================================

struct PhoCastSource {
    _source: usize, // raw pointer to obs_source_t stored as usize for thread safety
    ip: Arc<Mutex<String>>,
    port: Arc<AtomicU16>,
    running: Arc<AtomicBool>,
    width: Arc<AtomicU32>,
    height: Arc<AtomicU32>,
    thread_handle: Option<JoinHandle<()>>,
}

impl PhoCastSource {
    fn new(source_ptr: *mut c_void, ip: String, port: u16) -> Box<Self> {
        let ip_arc = Arc::new(Mutex::new(ip));
        let port_arc = Arc::new(AtomicU16::new(port));
        let running_arc = Arc::new(AtomicBool::new(true));
        let width_arc = Arc::new(AtomicU32::new(1920));
        let height_arc = Arc::new(AtomicU32::new(1080));

        let s_ptr_num = source_ptr as usize;
        let t_ip = Arc::clone(&ip_arc);
        let t_port = Arc::clone(&port_arc);
        let t_running = Arc::clone(&running_arc);
        let t_width = Arc::clone(&width_arc);
        let t_height = Arc::clone(&height_arc);

        let handle = thread::spawn(move || {
            worker_loop(s_ptr_num, t_ip, t_port, t_running, t_width, t_height);
        });

        Box::new(PhoCastSource {
            _source: s_ptr_num,
            ip: ip_arc,
            port: port_arc,
            running: running_arc,
            width: width_arc,
            height: height_arc,
            thread_handle: Some(handle),
        })
    }

    fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.thread_handle.take() {
            let _ = h.join();
        }
    }
}

fn worker_loop(
    source_ptr_num: usize,
    ip_arc: Arc<Mutex<String>>,
    port_arc: Arc<AtomicU16>,
    running: Arc<AtomicBool>,
    out_width: Arc<AtomicU32>,
    out_height: Arc<AtomicU32>,
) {
    let mut header_buf = [0u8; 4];
    let mut frame_buf = Vec::with_capacity(512 * 1024);

    while running.load(Ordering::SeqCst) {
        let ip = {
            let guard = ip_arc.lock().unwrap();
            guard.clone()
        };
        let port = port_arc.load(Ordering::SeqCst);
        let addr_str = format!("{}:{}", ip, port);

        // Attempt TCP connection to PhoCast server (phone or local forwarder)
        let stream = match addr_str.parse::<SocketAddr>() {
            Ok(addr) => TcpStream::connect_timeout(&addr, Duration::from_millis(1500)),
            Err(_) => {
                // Try resolving hostname/ip
                TcpStream::connect(&addr_str)
            }
        };

        let mut stream = match stream {
            Ok(s) => {
                let _ = s.set_read_timeout(Some(Duration::from_millis(3000)));
                let _ = s.set_nodelay(true);
                s
            }
            Err(_) => {
                // Connection failed: sleep briefly and retry
                thread::sleep(Duration::from_millis(1000));
                continue;
            }
        };

        // Streaming loop while connected
        while running.load(Ordering::SeqCst) {
            // Read 4-byte frame length
            if let Err(_) = stream.read_exact(&mut header_buf) {
                break; // Reconnect on read failure
            }

            let frame_len = u32::from_be_bytes(header_buf) as usize;
            if frame_len == 0 || frame_len > 15_000_000 {
                break; // Invalid frame length, reconnect
            }

            // Read exact JPEG frame payload
            frame_buf.resize(frame_len, 0);
            if let Err(_) = stream.read_exact(&mut frame_buf) {
                break; // Reconnect on read failure
            }

            // Decode JPEG directly into RGBA using high-performance SIMD decoder
            let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
            let mut decoder = JpegDecoder::new_with_options(&frame_buf, options);

            match decoder.decode() {
                Ok(mut rgba_pixels) => {
                    if let Some(info) = decoder.info() {
                        let w = info.width as u32;
                        let h = info.height as u32;
                        out_width.store(w, Ordering::Relaxed);
                        out_height.store(h, Ordering::Relaxed);

                        unsafe {
                            if let (Some(output_fn), Some(time_fn)) =
                                (OBS_API.obs_source_output_video, OBS_API.os_gettime_ns)
                            {
                                let mut frame = ObsSourceFrame::default();
                                frame.data[0] = rgba_pixels.as_mut_ptr();
                                frame.linesize[0] = w * 4;
                                frame.width = w;
                                frame.height = h;
                                frame.timestamp = time_fn();
                                frame.format = VIDEO_FORMAT_RGBA;
                                frame.full_range = true;
                                frame.max_luminance = 10000;

                                output_fn(source_ptr_num as *mut c_void, &frame);
                            }
                        }
                    }
                }
                Err(_) => {
                    // Skip corrupted frame
                }
            }
        }

        // Connection dropped: wait a bit before attempting reconnect
        thread::sleep(Duration::from_millis(500));
    }
}

// ============================================================================
// OBS Source Callbacks
// ============================================================================

static SOURCE_ID: &[u8] = b"phocast_source\0";
static SOURCE_NAME: &[u8] = b"PhoCast Camera\0";
static PROP_IP: &[u8] = b"ip\0";
static PROP_PORT: &[u8] = b"port\0";
static PROP_IP_DESC: &[u8] = b"Phone IP Address (127.0.0.1 for USB)\0";
static PROP_PORT_DESC: &[u8] = b"Port (Default: 5000)\0";

unsafe extern "C" fn phocast_get_name(_type_data: *mut c_void) -> *const c_char {
    SOURCE_NAME.as_ptr() as *const c_char
}

unsafe extern "C" fn phocast_create(settings: *mut c_void, source: *mut c_void) -> *mut c_void {
    let mut ip = "127.0.0.1".to_string();
    let mut port = 5000u16;

    if let Some(get_str) = OBS_API.obs_data_get_string {
        let p = get_str(settings, PROP_IP.as_ptr() as *const c_char);
        if !p.is_null() {
            if let Ok(s) = CStr::from_ptr(p).to_str() {
                if !s.is_empty() {
                    ip = s.to_string();
                }
            }
        }
    }

    if let Some(get_int) = OBS_API.obs_data_get_int {
        let val = get_int(settings, PROP_PORT.as_ptr() as *const c_char);
        if val > 0 && val <= 65535 {
            port = val as u16;
        }
    }

    let instance = PhoCastSource::new(source, ip, port);
    Box::into_raw(instance) as *mut c_void
}

unsafe extern "C" fn phocast_destroy(data: *mut c_void) {
    if !data.is_null() {
        let mut instance = Box::from_raw(data as *mut PhoCastSource);
        instance.stop();
    }
}

unsafe extern "C" fn phocast_get_width(data: *mut c_void) -> u32 {
    if data.is_null() {
        1920
    } else {
        let instance = &*(data as *mut PhoCastSource);
        instance.width.load(Ordering::Relaxed)
    }
}

unsafe extern "C" fn phocast_get_height(data: *mut c_void) -> u32 {
    if data.is_null() {
        1080
    } else {
        let instance = &*(data as *mut PhoCastSource);
        instance.height.load(Ordering::Relaxed)
    }
}

unsafe extern "C" fn phocast_get_defaults(settings: *mut c_void) {
    if let (Some(set_str), Some(set_int)) =
        (OBS_API.obs_data_set_default_string, OBS_API.obs_data_set_default_int)
    {
        let default_ip = CString::new("127.0.0.1").unwrap();
        set_str(settings, PROP_IP.as_ptr() as *const c_char, default_ip.as_ptr());
        set_int(settings, PROP_PORT.as_ptr() as *const c_char, 5000);
    }
}

unsafe extern "C" fn phocast_get_properties(_data: *mut c_void) -> *mut c_void {
    if let (Some(create_props), Some(add_text), Some(add_int)) = (
        OBS_API.obs_properties_create,
        OBS_API.obs_properties_add_text,
        OBS_API.obs_properties_add_int,
    ) {
        let props = create_props();
        add_text(
            props,
            PROP_IP.as_ptr() as *const c_char,
            PROP_IP_DESC.as_ptr() as *const c_char,
            OBS_TEXT_DEFAULT,
        );
        add_int(
            props,
            PROP_PORT.as_ptr() as *const c_char,
            PROP_PORT_DESC.as_ptr() as *const c_char,
            1024,
            65535,
            1,
        );
        props
    } else {
        std::ptr::null_mut()
    }
}

unsafe extern "C" fn phocast_update(data: *mut c_void, settings: *mut c_void) {
    if data.is_null() || settings.is_null() {
        return;
    }
    let instance = &*(data as *mut PhoCastSource);

    if let Some(get_str) = OBS_API.obs_data_get_string {
        let p = get_str(settings, PROP_IP.as_ptr() as *const c_char);
        if !p.is_null() {
            if let Ok(s) = CStr::from_ptr(p).to_str() {
                let mut guard = instance.ip.lock().unwrap();
                *guard = s.to_string();
            }
        }
    }

    if let Some(get_int) = OBS_API.obs_data_get_int {
        let val = get_int(settings, PROP_PORT.as_ptr() as *const c_char);
        if val > 0 && val <= 65535 {
            instance.port.store(val as u16, Ordering::SeqCst);
        }
    }
}

static PHOCAST_SOURCE_INFO: ObsSourceInfo = ObsSourceInfo {
    id: SOURCE_ID.as_ptr() as *const c_char,
    r#type: OBS_SOURCE_TYPE_INPUT,
    output_flags: OBS_SOURCE_ASYNC_VIDEO,
    get_name: Some(phocast_get_name),
    create: Some(phocast_create),
    destroy: Some(phocast_destroy),
    get_width: Some(phocast_get_width),
    get_height: Some(phocast_get_height),
    get_defaults: Some(phocast_get_defaults),
    get_properties: Some(phocast_get_properties),
    update: Some(phocast_update),
    activate: None,
    deactivate: None,
    show: None,
    hide: None,
    video_tick: None,
    video_render: None,
    filter_video: std::ptr::null_mut(),
    filter_audio: std::ptr::null_mut(),
    enum_active_sources: std::ptr::null_mut(),
    save: None,
    load: None,
    mouse_click: std::ptr::null_mut(),
    mouse_move: std::ptr::null_mut(),
    mouse_wheel: std::ptr::null_mut(),
    focus: std::ptr::null_mut(),
    key_click: std::ptr::null_mut(),
    filter_remove: std::ptr::null_mut(),
    type_data: std::ptr::null_mut(),
    free_type_data: std::ptr::null_mut(),
    audio_render: std::ptr::null_mut(),
    enum_all_sources: std::ptr::null_mut(),
    transition_start: std::ptr::null_mut(),
    transition_stop: std::ptr::null_mut(),
    get_defaults2: std::ptr::null_mut(),
    get_properties2: std::ptr::null_mut(),
    audio_mix: std::ptr::null_mut(),
    icon_type: OBS_ICON_TYPE_CAMERA,
    media_play_pause: std::ptr::null_mut(),
    media_restart: std::ptr::null_mut(),
    media_stop: std::ptr::null_mut(),
    media_next: std::ptr::null_mut(),
    media_previous: std::ptr::null_mut(),
    media_get_duration: std::ptr::null_mut(),
    media_get_time: std::ptr::null_mut(),
    media_set_time: std::ptr::null_mut(),
    media_get_state: std::ptr::null_mut(),
    version: 0,
    unversioned_id: std::ptr::null(),
    missing_files: std::ptr::null_mut(),
    video_get_color_space: std::ptr::null_mut(),
    filter_add: std::ptr::null_mut(),
    get_dark_icon: std::ptr::null_mut(),
    get_light_icon: std::ptr::null_mut(),
};

// ============================================================================
// OBS Module Exports
// ============================================================================

#[no_mangle]
pub unsafe extern "C" fn obs_module_ver() -> u32 {
    init_obs_api();
    if let Some(ver_fn) = OBS_API.obs_get_version {
        ver_fn()
    } else {
        (33 << 24) | (0 << 16) | 1
    }
}

#[no_mangle]
pub unsafe extern "C" fn obs_module_set_pointer(module: *mut c_void) {
    OBS_MODULE = module;
}

#[no_mangle]
pub unsafe extern "C" fn obs_module_load() -> bool {
    init_obs_api();
    if let Some(register) = OBS_API.obs_register_source_s {
        register(
            &PHOCAST_SOURCE_INFO,
            std::mem::size_of::<ObsSourceInfo>(),
        );
        true
    } else {
        false
    }
}

#[no_mangle]
pub unsafe extern "C" fn obs_module_unload() {}
