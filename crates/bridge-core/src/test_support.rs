use std::path::{Path, PathBuf};
pub struct Directory(pub PathBuf);
impl Directory {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "bridge-rust-{}",
            crate::sessions::random_id().unwrap()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub fn write(&self, name: &str, value: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, value).unwrap();
        path
    }
    pub fn dir(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
pub fn key(path: &Path) -> String {
    crate::paths::project_on(&path.to_string_lossy(), cfg!(windows)).unwrap()
}
