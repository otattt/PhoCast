use std::ffi::c_void;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::Memory::{
    MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, FILE_MAP_WRITE, MEMORY_MAPPED_VIEW_ADDRESS,
};
use windows_sys::Win32::System::Threading::{
    OpenEventW, OpenMutexW, ReleaseMutex, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE,
    INFINITE,
};

const SYNCHRONIZE: u32 = 0x0010_0000;

#[repr(C)]
pub struct SharedMemHeader {
    pub max_size: u32,
    pub width: i32,
    pub height: i32,
    pub stride: i32,
    pub format: i32,
    pub resizemode: i32,
    pub mirrormode: i32,
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
            // Check possible slot names: UnityCapture_Data, UnityCapture_Data0, UnityCapture_Data1
            let name_pairs = [
                ("UnityCapture_Mutx", "UnityCapture_Sent", "UnityCapture_Data"),
                ("UnityCapture_Mutx0", "UnityCapture_Sent0", "UnityCapture_Data0"),
                ("UnityCapture_Mutx1", "UnityCapture_Sent1", "UnityCapture_Data1"),
            ];

            for (mutx_name, sent_name, data_name) in name_pairs {
                let data_w = to_wide(data_name);
                let h_file = OpenFileMappingW(FILE_MAP_WRITE, 0, data_w.as_ptr());
                if !h_file.is_null() {
                    let mutx_w = to_wide(mutx_name);
                    let sent_w = to_wide(sent_name);

                    let h_mutex = OpenMutexW(SYNCHRONIZE, 0, mutx_w.as_ptr());
                    let h_sent = OpenEventW(EVENT_MODIFY_STATE, 0, sent_w.as_ptr());

                    if !h_mutex.is_null() && !h_sent.is_null() {
                        let view = MapViewOfFile(h_file, FILE_MAP_WRITE, 0, 0, 0);
                        if !view.Value.is_null() {
                            let header = view.Value as *mut SharedMemHeader;
                            let data_ptr = (view.Value as usize + std::mem::size_of::<SharedMemHeader>()) as *mut u8;
                            return Some(Self {
                                h_mutex,
                                h_sent,
                                h_file,
                                header,
                                data_ptr,
                            });
                        }
                    }
                    if !h_mutex.is_null() { CloseHandle(h_mutex); }
                    if !h_sent.is_null() { CloseHandle(h_sent); }
                    CloseHandle(h_file);
                }
            }
            None
        }
    }

    pub fn send_frame(&mut self, width: usize, height: usize, rgba_bytes: &[u8]) {
        unsafe {
            if self.header.is_null() { return; }

            WaitForSingleObject(self.h_mutex, INFINITE);

            (*self.header).width = width as i32;
            (*self.header).height = height as i32;
            (*self.header).stride = (width * 4) as i32;
            (*self.header).format = 0; // FORMAT_UINT8 (RGBA)
            (*self.header).resizemode = 1; // RESIZEMODE_LINEAR
            (*self.header).mirrormode = 0;
            (*self.header).timeout = 1000;

            let max_data = (*self.header).max_size as usize;
            let copy_len = rgba_bytes.len().min(if max_data > 0 { max_data } else { rgba_bytes.len() });
            std::ptr::copy_nonoverlapping(rgba_bytes.as_ptr(), self.data_ptr, copy_len);

            ReleaseMutex(self.h_mutex);
            SetEvent(self.h_sent);
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
