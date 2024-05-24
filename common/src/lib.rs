pub mod model;
pub mod crypto;

#[cfg(unix)]
pub const PLATFORM: model::Platform = model::Platform::Unix;
#[cfg(windows)]
pub const PLATFORM: model::Platform = model::Platform::Windows;
pub const PLATFORM_HEADER: &str = "Platform";
