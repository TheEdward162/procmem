#[cfg(any(
	target_os = "linux",
	target_os = "macos"
))]
pub mod ptrace;

#[cfg(target_os = "linux")]
pub mod procfs;

#[cfg(target_os = "macos")]
pub mod mach;

#[cfg(feature = "platform_simple")]
pub mod simple;

// TODO: mach virtual memory api

// TODO: windows virtual memory api
