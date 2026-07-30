use log::{Level, Metadata, Record, SetLoggerError};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(dead_code)]
pub struct FileLogger {
    file: Mutex<File>,
}

#[allow(dead_code)]
impl FileLogger {
    pub fn new(path: &str) -> std::io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            file: Mutex::new(file),
        })
    }
}

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();

            let mut file = self.file.lock().unwrap();
            let _ = writeln!(
                file,
                "[{}][{}][{}] {}",
                now,
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {
        let mut file = self.file.lock().unwrap();
        let _ = file.flush();
    }
}

#[allow(dead_code)]
pub fn init_logger() -> Result<(), SetLoggerError> {
    // Determine log path (e.g., next to executable or in temp)
    // For simplicity/robustness in this environment, try root specific or relative
    // In production, should be AppData/Logs
    let log_path = "vibe_diagnostics.log";

    // Clear previous log
    let _ = std::fs::remove_file(log_path);

    let logger = FileLogger::new(log_path).expect("Failed to create log file");

    log::set_boxed_logger(Box::new(logger)).map(|()| log::set_max_level(log::LevelFilter::Trace))
}

#[allow(dead_code)]
pub fn log_panic(info: &std::panic::PanicHookInfo) {
    let msg = format!("CRITICAL PANIC: {:?}", info);
    eprintln!("{}", msg);
    log::error!("{}", msg);
}
