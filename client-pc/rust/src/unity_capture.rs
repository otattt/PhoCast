use std::ffi::c_void;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, FILE_MAP_ALL_ACCESS,
    MEMORY_MAPPED_VIEW_ADDRESS, PAGE_READWRITE,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateMutexW, OpenEventW, OpenMutexW, ReleaseMutex, SetEvent, WaitForSingleObject,
    EVENT_ALL_ACCESS, MUTEX_ALL_ACCESS,
};

const TOTAL_SIZE: u32 = 70778880; // 0x4380000 (UnityCapture standard buffer: 4K FP16 + header)
const MAX_IMAGE_SIZE: u32 = 66355200; // 0x3f48000

#[repr(C)]
pub struct SharedMemHeader {
    pub max_size: u32,
    pub width: i32,
    pub height: i32,
    pub stride: i32,
    pub format: i32,      // 0 = FORMAT_UINT8 (RGBA)
    pub resizemode: i32,  // 1 = RESIZEMODE_LINEAR
    pub mirrormode: i32,  // 0 = DISABLED
    pub timeout: i32,
}

pub struct PhoCastCameraWriter {
    h_mutex: HANDLE,
    h_sent: HANDLE,
    h_file: HANDLE,
    header: *mut SharedMemHeader,
    data_ptr: *mut u8,
}

unsafe impl Send for PhoCastCameraWriter {}
unsafe impl Sync for PhoCastCameraWriter {}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

impl PhoCastCameraWriter {
    pub fn try_open() -> Option<Self> {
        unsafe {
            let data_w = to_wide("UnityCapture_Data0");
            let mutx_w = to_wide("UnityCapture_Mutx0");
            let sent_w = to_wide("UnityCapture_Sent0");

            // 1. Open or create shared memory file mapping
            let mut h_file = OpenFileMappingW(FILE_MAP_ALL_ACCESS, 0, data_w.as_ptr());
            if h_file.is_null() {
                h_file = CreateFileMappingW(
                    INVALID_HANDLE_VALUE,
                    std::ptr::null(),
                    PAGE_READWRITE,
                    0,
                    TOTAL_SIZE,
                    data_w.as_ptr(),
                );
            }

            if h_file.is_null() {
                return None;
            }

            // 2. Open or create Mutex
            let mut h_mutex = OpenMutexW(MUTEX_ALL_ACCESS, 0, mutx_w.as_ptr());
            if h_mutex.is_null() {
                h_mutex = CreateMutexW(std::ptr::null(), 0, mutx_w.as_ptr());
            }

            // 3. Open or create Sent Event
            let mut h_sent = OpenEventW(EVENT_ALL_ACCESS, 0, sent_w.as_ptr());
            if h_sent.is_null() {
                h_sent = CreateEventW(std::ptr::null(), 0, 0, sent_w.as_ptr());
            }

            // 4. Map the memory view into our address space
            let view = MapViewOfFile(h_file, FILE_MAP_ALL_ACCESS, 0, 0, 0);
            if !view.Value.is_null() {
                let header = view.Value as *mut SharedMemHeader;
                (*header).max_size = MAX_IMAGE_SIZE;
                (*header).format = 0; // RGBA
                (*header).resizemode = 1; // Linear
                (*header).mirrormode = 0;
                (*header).timeout = 1000;

                let data_ptr = (view.Value as usize + std::mem::size_of::<SharedMemHeader>()) as *mut u8;

                Some(Self {
                    h_mutex,
                    h_sent,
                    h_file,
                    header,
                    data_ptr,
                })
            } else {
                if !h_mutex.is_null() { CloseHandle(h_mutex); }
                if !h_sent.is_null() { CloseHandle(h_sent); }
                CloseHandle(h_file);
                None
            }
        }
    }

    pub fn send_frame(&mut self, width: usize, height: usize, rgba_bytes: &[u8]) {
        unsafe {
            if self.header.is_null() || self.data_ptr.is_null() { return; }

            if !self.h_mutex.is_null() {
                WaitForSingleObject(self.h_mutex, 50);
            }

            (*self.header).width = width as i32;
            (*self.header).height = height as i32;
            (*self.header).stride = (width * 4) as i32;
            (*self.header).format = 0; // RGBA
            (*self.header).resizemode = 1;
            (*self.header).mirrormode = 0;
            (*self.header).timeout = 1000;

            let max_data = (*self.header).max_size as usize;
            let copy_len = rgba_bytes.len().min(if max_data > 0 { max_data } else { rgba_bytes.len() });
            std::ptr::copy_nonoverlapping(rgba_bytes.as_ptr(), self.data_ptr, copy_len);

            if !self.h_mutex.is_null() {
                ReleaseMutex(self.h_mutex);
            }
            if !self.h_sent.is_null() {
                SetEvent(self.h_sent);
            }
        }
    }
}

impl Drop for PhoCastCameraWriter {
    fn drop(&mut self) {
        unsafe {
            if !self.header.is_null() {
                UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS { Value: self.header as *mut c_void });
            }
            if !self.h_file.is_null() { CloseHandle(self.h_file); }
            if !self.h_mutex.is_null() { CloseHandle(self.h_mutex); }
            if !self.h_sent.is_null() { CloseHandle(self.h_sent); }
        }
    }
}
