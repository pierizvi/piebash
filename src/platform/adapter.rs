// Platform adapter - placeholder for future use
pub trait PlatformAdapter {
    fn get_home_dir(&self) -> std::path::PathBuf;
    fn get_temp_dir(&self) -> std::path::PathBuf;
}