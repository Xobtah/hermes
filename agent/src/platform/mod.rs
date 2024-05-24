#[cfg(unix)]
#[path = "linux.rs"]
mod native;
#[cfg(windows)]
#[path = "windows.rs"]
mod native;

// #[cfg(any(unix, windows))]
pub use native::*;