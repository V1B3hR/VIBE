use crate::engine::streamer::pool::AudioBlock;
use crossbeam_channel::Sender;

use std::path::Path;
use std::ptr::null_mut;
use std::sync::Arc;
use std::thread;
use winapi::um::fileapi::OPEN_EXISTING;
use winapi::um::fileapi::{CreateFileW, ReadFile};
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::ioapiset::{
    CreateIoCompletionPort, GetQueuedCompletionStatus, PostQueuedCompletionStatus,
};
use winapi::um::minwinbase::OVERLAPPED;
use winapi::um::winbase::{FILE_FLAG_NO_BUFFERING, FILE_FLAG_OVERLAPPED};
use winapi::um::winnt::{FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, GENERIC_READ, HANDLE};

/// A request for a chunk of audio from disk.
pub struct StreamRequest {
    pub file_handle: HANDLE,
    pub offset: u64,
    pub target_block: Box<AudioBlock>,
    pub callback_tx: Sender<Box<AudioBlock>>,
}

#[repr(C)]
struct InternalRequest {
    overlapped: OVERLAPPED,
    request: StreamRequest,
}

/// Windows-specific high-performance async streamer using IOCP.
pub struct WindowsAsyncStreamer {
    iocp: HANDLE,
    _worker_threads: Vec<thread::JoinHandle<()>>,
}

unsafe impl Send for WindowsAsyncStreamer {}
unsafe impl Sync for WindowsAsyncStreamer {}

impl WindowsAsyncStreamer {
    pub fn new(num_threads: usize) -> Arc<Self> {
        unsafe {
            let iocp =
                CreateIoCompletionPort(INVALID_HANDLE_VALUE, null_mut(), 0, num_threads as u32);
            if iocp.is_null() {
                panic!("Failed to create IOCP");
            }

            let streamer = Arc::new(Self {
                iocp,
                _worker_threads: Vec::new(),
            });

            // We can't easily store JoinHandles in a self-referential way if they need the Arc
            // but we can spawn them and they'll run as long as the Arc (and IOCP) exists.
            for _ in 0..num_threads {
                let iocp_handle = iocp as usize;
                thread::spawn(move || {
                    Self::worker_thread_fn(iocp_handle as HANDLE);
                });
            }

            streamer
        }
    }

    /// Open a file optimized for high-performance audio streaming.
    #[allow(dead_code)]
    pub fn open_file<P: AsRef<Path>>(path: P) -> Result<HANDLE, String> {
        use std::os::windows::ffi::OsStrExt;
        let wide_path: Vec<u16> = path
            .as_ref()
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let handle = CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ,
                null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OVERLAPPED | FILE_FLAG_NO_BUFFERING,
                null_mut(),
            );

            if handle == INVALID_HANDLE_VALUE {
                return Err(format!("Failed to open file: {:?}", path.as_ref()));
            }
            Ok(handle)
        }
    }

    /// Associate a file handle with this streamer's Completion Port.
    #[allow(dead_code)]
    pub fn register_file(&self, handle: HANDLE) -> Result<(), String> {
        unsafe {
            let res = CreateIoCompletionPort(handle, self.iocp, handle as usize, 0);
            if res.is_null() {
                return Err("Failed to register file with IOCP".to_string());
            }
            Ok(())
        }
    }

    /// Submit an asynchronous read request.
    pub fn read_at(&self, request: StreamRequest) -> Result<(), String> {
        unsafe {
            let mut internal = Box::new(InternalRequest {
                overlapped: std::mem::zeroed(),
                request,
            });

            // Set the offset in the overlapped structure
            let s = internal.overlapped.u.s_mut();
            s.Offset = (internal.request.offset & 0xFFFFFFFF) as u32;
            s.OffsetHigh = (internal.request.offset >> 32) as u32;

            let buffer_ptr =
                internal.request.target_block.data.as_mut_ptr() as *mut winapi::ctypes::c_void;
            let buffer_len = (internal.request.target_block.data.len() * 4) as u32;

            let internal_ptr = Box::into_raw(internal);

            let res = ReadFile(
                (*internal_ptr).request.file_handle,
                buffer_ptr,
                buffer_len,
                null_mut(),
                &mut (*internal_ptr).overlapped,
            );

            if res == 0 {
                let err = winapi::um::errhandlingapi::GetLastError();
                if err != winapi::shared::winerror::ERROR_IO_PENDING {
                    // Immediate failure, reclaim the box
                    let _ = Box::from_raw(internal_ptr);
                    return Err(format!("ReadFile failed with error code: {}", err));
                }
            }
            Ok(())
        }
    }

    fn worker_thread_fn(iocp: HANDLE) {
        unsafe {
            let mut completion_key: usize = 0;
            let mut overlapped_ptr: *mut OVERLAPPED = null_mut();
            let mut bytes_transferred: u32 = 0;

            loop {
                let res = GetQueuedCompletionStatus(
                    iocp,
                    &mut bytes_transferred,
                    &mut completion_key,
                    &mut overlapped_ptr,
                    winapi::um::winbase::INFINITE,
                );

                if overlapped_ptr.is_null() {
                    break; // Shutdown signal
                }

                // Retrieve the InternalRequest from the overlapped pointer
                let internal_ptr = overlapped_ptr as *mut InternalRequest;
                let internal = Box::from_raw(internal_ptr);

                if res == 0 {
                    // Operation failed or was cancelled
                    // For now, we just drop it (which returns the block to nobody,
                    // eventually we should handle errors and notify the scheduler)
                    println!("IOCP Operation failed");
                    continue;
                }

                // Operation successful. Send the filled block back to the caller.
                let _ = internal
                    .request
                    .callback_tx
                    .send(internal.request.target_block);
            }
        }
    }

    pub fn shutdown(&self) {
        unsafe {
            // Send one null overlapped per worker thread (simplification)
            for _ in 0..16 {
                // Assuming max 16 threads for now
                PostQueuedCompletionStatus(self.iocp, 0, 0, null_mut());
            }
        }
    }
}

impl Drop for WindowsAsyncStreamer {
    fn drop(&mut self) {
        self.shutdown();
        unsafe {
            CloseHandle(self.iocp);
        }
    }
}
