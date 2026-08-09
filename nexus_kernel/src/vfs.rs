use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

/// Metadata for a file
pub struct FileMeta {
    pub content: Vec<u8>,
    pub size: usize,
    pub created_tick: u64,
}

/// Metadata for a directory
pub struct DirEntry {
    pub files: BTreeMap<String, FileMeta>,
    pub subdirs: BTreeMap<String, DirEntry>,
}

impl DirEntry {
    pub fn new() -> Self {
        DirEntry {
            files: BTreeMap::new(),
            subdirs: BTreeMap::new(),
        }
    }
}

/// A hierarchical in-memory filesystem
pub struct Ramfs {
    pub root: DirEntry,
    pub cwd: String, // current working directory path
}

impl Ramfs {
    pub fn new() -> Self {
        Ramfs {
            root: DirEntry::new(),
            cwd: String::from("/"),
        }
    }

    /// Navigate to a directory by path, returning a reference
    fn navigate(&self, path: &str) -> Option<&DirEntry> {
        let resolved = self.resolve_path(path);
        let mut current = &self.root;
        if resolved == "/" {
            return Some(current);
        }
        for part in resolved.trim_start_matches('/').split('/') {
            if part.is_empty() { continue; }
            current = current.subdirs.get(part)?;
        }
        Some(current)
    }

    /// Navigate to a directory by path, returning a mutable reference
    fn navigate_mut(&mut self, path: &str) -> Option<&mut DirEntry> {
        let resolved = self.resolve_path(path);
        let mut current = &mut self.root;
        if resolved == "/" {
            return Some(current);
        }
        for part in resolved.trim_start_matches('/').split('/') {
            if part.is_empty() { continue; }
            current = current.subdirs.get_mut(part)?;
        }
        Some(current)
    }

    /// Resolve a path (handle relative vs absolute)
    fn resolve_path(&self, path: &str) -> String {
        if path.starts_with('/') {
            String::from(path)
        } else {
            let mut full = self.cwd.clone();
            if !full.ends_with('/') {
                full.push('/');
            }
            full.push_str(path);
            full
        }
    }

    /// Split a path into (parent_path, basename)
    fn split_path(&self, path: &str) -> (String, String) {
        let resolved = self.resolve_path(path);
        if let Some(pos) = resolved.rfind('/') {
            let parent = if pos == 0 { String::from("/") } else { String::from(&resolved[..pos]) };
            let name = String::from(&resolved[pos + 1..]);
            (parent, name)
        } else {
            (self.cwd.clone(), resolved)
        }
    }

    pub fn mkdir(&mut self, path: &str) -> Result<(), &'static str> {
        let (parent_path, dir_name) = self.split_path(path);
        if dir_name.is_empty() {
            return Err("Invalid directory name");
        }
        let parent = self.navigate_mut(&parent_path).ok_or("Parent directory not found")?;
        if parent.subdirs.contains_key(&dir_name) {
            return Err("Directory already exists");
        }
        parent.subdirs.insert(dir_name, DirEntry::new());
        Ok(())
    }

    pub fn create_file(&mut self, path: &str) -> Result<(), &'static str> {
        let (parent_path, file_name) = self.split_path(path);
        if file_name.is_empty() {
            return Err("Invalid file name");
        }
        let parent = self.navigate_mut(&parent_path).ok_or("Parent directory not found")?;
        if !parent.files.contains_key(&file_name) {
            parent.files.insert(file_name, FileMeta {
                content: Vec::new(),
                size: 0,
                created_tick: 0,
            });
        }
        Ok(())
    }

    pub fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), &'static str> {
        let (parent_path, file_name) = self.split_path(path);
        let parent = self.navigate_mut(&parent_path).ok_or("Parent directory not found")?;
        let file = parent.files.get_mut(&file_name).ok_or("File not found")?;
        file.content.extend_from_slice(data);
        file.size = file.content.len();
        Ok(())
    }

    pub fn read_file(&self, path: &str) -> Result<&[u8], &'static str> {
        let (parent_path, file_name) = self.split_path(path);
        let parent = self.navigate(&parent_path).ok_or("Parent directory not found")?;
        let file = parent.files.get(&file_name).ok_or("File not found")?;
        Ok(file.content.as_slice())
    }

    pub fn delete_file(&mut self, path: &str) -> Result<(), &'static str> {
        let (parent_path, file_name) = self.split_path(path);
        let parent = self.navigate_mut(&parent_path).ok_or("Parent directory not found")?;
        parent.files.remove(&file_name).ok_or("File not found")?;
        Ok(())
    }

    pub fn rmdir(&mut self, path: &str) -> Result<(), &'static str> {
        let (parent_path, dir_name) = self.split_path(path);
        let parent = self.navigate_mut(&parent_path).ok_or("Parent directory not found")?;
        let dir = parent.subdirs.get(&dir_name).ok_or("Directory not found")?;
        if !dir.files.is_empty() || !dir.subdirs.is_empty() {
            return Err("Directory not empty");
        }
        parent.subdirs.remove(&dir_name);
        Ok(())
    }

    pub fn list_dir(&self, path: &str) -> Result<(Vec<String>, Vec<String>), &'static str> {
        let dir = self.navigate(path).ok_or("Directory not found")?;
        let dirs: Vec<String> = dir.subdirs.keys().cloned().collect();
        let files: Vec<String> = dir.files.keys().cloned().collect();
        Ok((dirs, files))
    }

    pub fn cd(&mut self, path: &str) -> Result<(), &'static str> {
        if path == ".." {
            // Go up one level
            if self.cwd == "/" {
                return Ok(());
            }
            if let Some(pos) = self.cwd.rfind('/') {
                if pos == 0 {
                    self.cwd = String::from("/");
                } else {
                    self.cwd = String::from(&self.cwd[..pos]);
                }
            }
            return Ok(());
        }
        let resolved = self.resolve_path(path);
        // Verify the directory exists
        if self.navigate(&resolved).is_some() {
            self.cwd = resolved;
            Ok(())
        } else {
            Err("Directory not found")
        }
    }

    pub fn pwd(&self) -> &str {
        &self.cwd
    }

    pub fn file_info(&self, path: &str) -> Result<(usize,), &'static str> {
        let (parent_path, file_name) = self.split_path(path);
        let parent = self.navigate(&parent_path).ok_or("Parent directory not found")?;
        let file = parent.files.get(&file_name).ok_or("File not found")?;
        Ok((file.size,))
    }
}

lazy_static! {
    pub static ref ROOT_FS: Mutex<Ramfs> = Mutex::new(Ramfs::new());
}
