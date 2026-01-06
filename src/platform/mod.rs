pub mod paths;
pub mod adapter;

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

pub fn detect_platform() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    format!("{}-{}", os, arch)
}