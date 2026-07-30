pub mod pool;
pub mod reader;
pub mod windows_streamer;

pub use pool::GlobalBufferPool;
// pub use pool::AudioBlock;
// pub use reader::HyperStreamReader;
// Re-check why compiler thinks it is unused. Maybe it is not used directly via `streamer::`.
// I will just keep GlobalBufferPool and WindowsAsyncStreamer for now if those are the only ones used.
// Actually, let's keep it clean.
pub use windows_streamer::WindowsAsyncStreamer;
