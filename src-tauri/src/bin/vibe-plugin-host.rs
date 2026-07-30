// Vibe Plugin Host - Separate process for plugin isolation
// This executable runs plugins in complete isolation from the main DAW

use std::io::{Read, Write};
use std::os::windows::io::FromRawHandle;

#[repr(C)]
#[allow(dead_code)]
struct HostMessage {
    command: u32,
    data_len: u32,
}

const CMD_LOAD_PLUGIN: u32 = 1;
const CMD_PROCESS_AUDIO: u32 = 2;
const CMD_SHUTDOWN: u32 = 3;
#[allow(dead_code)]
const CMD_SET_PARAMETER: u32 = 4;

fn main() {
    println!("VIBE Plugin Host V2 - Starting...");

    // Get pipe name from command line
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: vibe-plugin-host <pipe_name>");
        std::process::exit(1);
    }

    let pipe_name = &args[1];
    println!("Connecting to pipe: {}", pipe_name);

    // Connect to named pipe from main process
    match connect_to_pipe(pipe_name) {
        Ok(mut pipe) => {
            println!("Connected to main process");

            // Main message loop
            loop {
                let mut header = [0u8; 8];
                if pipe.read_exact(&mut header).is_err() {
                    println!("Pipe closed, shutting down");
                    break;
                }

                let command = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
                let data_len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);

                match command {
                    CMD_LOAD_PLUGIN => {
                        let mut path_bytes = vec![0u8; data_len as usize];
                        pipe.read_exact(&mut path_bytes).unwrap();
                        let path = String::from_utf8_lossy(&path_bytes);
                        println!("Loading plugin: {}", path);

                        // TODO: Actually load plugin (VST2/VST3/WASM)
                        // For now, send success response
                        pipe.write_all(&[1u8]).unwrap();
                    }
                    CMD_PROCESS_AUDIO => {
                        // TODO: Process audio through loaded plugin
                        // For now, just acknowledge
                        pipe.write_all(&[1u8]).unwrap();
                    }
                    CMD_SHUTDOWN => {
                        println!("Shutdown requested");
                        break;
                    }
                    _ => {
                        eprintln!("Unknown command: {}", command);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to pipe: {}", e);
            std::process::exit(1);
        }
    }

    println!("Plugin host exiting");
}

#[cfg(target_os = "windows")]
fn connect_to_pipe(pipe_name: &str) -> Result<std::fs::File, String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::fileapi::CreateFileW;
    use winapi::um::winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE};

    let pipe_path = format!(r"\\.\pipe\{}", pipe_name);
    let wide: Vec<u16> = OsStr::new(&pipe_path)
        .encode_wide()
        .chain(Some(0))
        .collect();

    unsafe {
        let handle = CreateFileW(
            wide.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            3, // OPEN_EXISTING
            0,
            std::ptr::null_mut(),
        );

        if handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            return Err("Failed to open pipe".to_string());
        }

        Ok(std::fs::File::from_raw_handle(handle as *mut _))
    }
}

#[cfg(not(target_os = "windows"))]
fn connect_to_pipe(_pipe_name: &str) -> Result<std::fs::File, String> {
    Err("Named pipes only supported on Windows".to_string())
}
