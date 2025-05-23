//! Interactions with the file system.
use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::str;
use std::time::SystemTime;
use xi_rope::Rope;
use xi_rpc::RemoteError;
use crate::tabs::BufferId;
#[cfg(feature = "notify")]
use crate::tabs::OPEN_FILE_EVENT_TOKEN;
#[cfg(feature = "notify")]
use crate::watcher::FileWatcher;
#[cfg(target_family = "unix")]
use std::{fs::Permissions, os::unix::fs::PermissionsExt};
const UTF8_BOM: &str = "\u{feff}";
/// Tracks all state related to open files.
pub struct FileManager {
    open_files: HashMap<PathBuf, BufferId>,
    file_info: HashMap<BufferId, FileInfo>,
    /// A monitor of filesystem events, for things like reloading changed files.
    #[cfg(feature = "notify")]
    watcher: FileWatcher,
}
#[derive(Debug)]
pub struct FileInfo {
    pub encoding: CharacterEncoding,
    pub path: PathBuf,
    pub mod_time: Option<SystemTime>,
    pub has_changed: bool,
    #[cfg(target_family = "unix")]
    pub permissions: Option<u32>,
}
pub enum FileError {
    Io(io::Error, PathBuf),
    UnknownEncoding(PathBuf),
    HasChanged(PathBuf),
}
#[derive(Debug, Clone, Copy)]
pub enum CharacterEncoding {
    Utf8,
    Utf8WithBom,
}
impl FileManager {
    #[cfg(feature = "notify")]
    pub fn new(watcher: FileWatcher) -> Self {
        FileManager {
            open_files: HashMap::new(),
            file_info: HashMap::new(),
            watcher,
        }
    }
    #[cfg(not(feature = "notify"))]
    pub fn new() -> Self {
        FileManager {
            open_files: HashMap::new(),
            file_info: HashMap::new(),
        }
    }
    #[cfg(feature = "notify")]
    pub fn watcher(&mut self) -> &mut FileWatcher {
        &mut self.watcher
    }
    pub fn get_info(&self, id: BufferId) -> Option<&FileInfo> {
        self.file_info.get(&id)
    }
    pub fn get_editor(&self, path: &Path) -> Option<BufferId> {
        self.open_files.get(path).cloned()
    }
    /// Returns `true` if this file is open and has changed on disk.
    /// This state is stashed.
    pub fn check_file(&mut self, path: &Path, id: BufferId) -> bool {
        if let Some(info) = self.file_info.get_mut(&id) {
            let mod_t = get_mod_time(path);
            if mod_t != info.mod_time {
                info.has_changed = true;
            }
            return info.has_changed;
        }
        false
    }
    pub fn open(&mut self, path: &Path, id: BufferId) -> Result<Rope, FileError> {
        if !path.exists() {
            return Ok(Rope::from(""));
        }
        let (rope, info) = try_load_file(path)?;
        self.open_files.insert(path.to_owned(), id);
        if self.file_info.insert(id, info).is_none() {
            #[cfg(feature = "notify")]
            self.watcher.watch(path, false, OPEN_FILE_EVENT_TOKEN);
        }
        Ok(rope)
    }
    pub fn close(&mut self, id: BufferId) {
        if let Some(info) = self.file_info.remove(&id) {
            self.open_files.remove(&info.path);
            #[cfg(feature = "notify")]
            self.watcher.unwatch(&info.path, OPEN_FILE_EVENT_TOKEN);
        }
    }
    pub fn save(
        &mut self,
        path: &Path,
        text: &Rope,
        id: BufferId,
    ) -> Result<(), FileError> {
        let is_existing = self.file_info.contains_key(&id);
        if is_existing {
            self.save_existing(path, text, id)
        } else {
            self.save_new(path, text, id)
        }
    }
    fn save_new(
        &mut self,
        path: &Path,
        text: &Rope,
        id: BufferId,
    ) -> Result<(), FileError> {
        try_save(path, text, CharacterEncoding::Utf8, self.get_info(id))
            .map_err(|e| FileError::Io(e, path.to_owned()))?;
        let info = FileInfo {
            encoding: CharacterEncoding::Utf8,
            path: path.to_owned(),
            mod_time: get_mod_time(path),
            has_changed: false,
            #[cfg(target_family = "unix")]
            permissions: get_permissions(path),
        };
        self.open_files.insert(path.to_owned(), id);
        self.file_info.insert(id, info);
        #[cfg(feature = "notify")]
        self.watcher.watch(path, false, OPEN_FILE_EVENT_TOKEN);
        Ok(())
    }
    fn save_existing(
        &mut self,
        path: &Path,
        text: &Rope,
        id: BufferId,
    ) -> Result<(), FileError> {
        let prev_path = self.file_info[&id].path.clone();
        if prev_path != path {
            self.save_new(path, text, id)?;
            self.open_files.remove(&prev_path);
            #[cfg(feature = "notify")]
            self.watcher.unwatch(&prev_path, OPEN_FILE_EVENT_TOKEN);
        } else if self.file_info[&id].has_changed {
            return Err(FileError::HasChanged(path.to_owned()));
        } else {
            let encoding = self.file_info[&id].encoding;
            try_save(path, text, encoding, self.get_info(id))
                .map_err(|e| FileError::Io(e, path.to_owned()))?;
            self.file_info.get_mut(&id).unwrap().mod_time = get_mod_time(path);
        }
        Ok(())
    }
}
fn try_load_file<P>(path: P) -> Result<(Rope, FileInfo), FileError>
where
    P: AsRef<Path>,
{
    let mut f = File::open(path.as_ref())
        .map_err(|e| FileError::Io(e, path.as_ref().to_owned()))?;
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes).map_err(|e| FileError::Io(e, path.as_ref().to_owned()))?;
    let encoding = CharacterEncoding::guess(&bytes);
    let rope = try_decode(bytes, encoding, path.as_ref())?;
    let info = FileInfo {
        encoding,
        mod_time: get_mod_time(&path),
        #[cfg(target_family = "unix")]
        permissions: get_permissions(&path),
        path: path.as_ref().to_owned(),
        has_changed: false,
    };
    Ok((rope, info))
}
#[allow(unused)]
fn try_save(
    path: &Path,
    text: &Rope,
    encoding: CharacterEncoding,
    file_info: Option<&FileInfo>,
) -> io::Result<()> {
    let tmp_extension = path
        .extension()
        .map_or_else(
            || OsString::from("swp"),
            |ext| {
                let mut ext = ext.to_os_string();
                ext.push(".swp");
                ext
            },
        );
    let tmp_path = &path.with_extension(tmp_extension);
    let mut f = File::create(tmp_path)?;
    match encoding {
        CharacterEncoding::Utf8WithBom => f.write_all(UTF8_BOM.as_bytes())?,
        CharacterEncoding::Utf8 => {}
    }
    for chunk in text.iter_chunks(..text.len()) {
        f.write_all(chunk.as_bytes())?;
    }
    fs::rename(tmp_path, path)?;
    #[cfg(target_family = "unix")]
    {
        if let Some(info) = file_info {
            fs::set_permissions(
                    path,
                    Permissions::from_mode(info.permissions.unwrap_or(0o644)),
                )
                .unwrap_or_else(|e| {
                    warn!(
                        "Couldn't set permissions on file {} due to error {}", path
                        .display(), e
                    )
                });
        }
    }
    Ok(())
}
fn try_decode(
    bytes: Vec<u8>,
    encoding: CharacterEncoding,
    path: &Path,
) -> Result<Rope, FileError> {
    match encoding {
        CharacterEncoding::Utf8 => {
            Ok(
                Rope::from(
                    str::from_utf8(&bytes)
                        .map_err(|_e| FileError::UnknownEncoding(path.to_owned()))?,
                ),
            )
        }
        CharacterEncoding::Utf8WithBom => {
            let s = String::from_utf8(bytes)
                .map_err(|_e| FileError::UnknownEncoding(path.to_owned()))?;
            Ok(Rope::from(&s[UTF8_BOM.len()..]))
        }
    }
}
impl CharacterEncoding {
    fn guess(s: &[u8]) -> Self {
        if s.starts_with(UTF8_BOM.as_bytes()) {
            CharacterEncoding::Utf8WithBom
        } else {
            CharacterEncoding::Utf8
        }
    }
}
/// Returns the modification timestamp for the file at a given path,
/// if present.
fn get_mod_time<P: AsRef<Path>>(path: P) -> Option<SystemTime> {
    File::open(path).and_then(|f| f.metadata()).and_then(|meta| meta.modified()).ok()
}
/// Returns the file permissions for the file at a given path on UNIXy systems,
/// if present.
#[cfg(target_family = "unix")]
fn get_permissions<P: AsRef<Path>>(path: P) -> Option<u32> {
    File::open(path)
        .and_then(|f| f.metadata())
        .map(|meta| meta.permissions().mode())
        .ok()
}
impl From<FileError> for RemoteError {
    fn from(src: FileError) -> RemoteError {
        let code = src.error_code();
        let message = src.to_string();
        RemoteError::custom(code, message, None)
    }
}
impl FileError {
    fn error_code(&self) -> i64 {
        match self {
            FileError::Io(_, _) => 5,
            FileError::UnknownEncoding(_) => 6,
            FileError::HasChanged(_) => 7,
        }
    }
}
impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileError::Io(ref e, ref p) => write!(f, "{}. File path: {:?}", e, p),
            FileError::UnknownEncoding(ref p) => {
                write!(f, "Error decoding file: {:?}", p)
            }
            FileError::HasChanged(ref p) => {
                write!(
                    f,
                    "File has changed on disk. \
                 Please save elsewhere and reload the file. File path: {:?}",
                    p
                )
            }
        }
    }
}
#[cfg(test)]
mod tests_llm_16_409 {
    use super::*;
    use crate::*;
    use std::io;
    use std::path::PathBuf;
    #[test]
    fn test_error_code_io_error() {
        let _rug_st_tests_llm_16_409_rrrruuuugggg_test_error_code_io_error = 0;
        let rug_fuzz_0 = "test.txt";
        let error = FileError::Io(
            io::Error::from(io::ErrorKind::NotFound),
            PathBuf::from(rug_fuzz_0),
        );
        debug_assert_eq!(error.error_code(), 5);
        let _rug_ed_tests_llm_16_409_rrrruuuugggg_test_error_code_io_error = 0;
    }
    #[test]
    fn test_error_code_unknown_encoding() {
        let _rug_st_tests_llm_16_409_rrrruuuugggg_test_error_code_unknown_encoding = 0;
        let rug_fuzz_0 = "test.txt";
        let error = FileError::UnknownEncoding(PathBuf::from(rug_fuzz_0));
        debug_assert_eq!(error.error_code(), 6);
        let _rug_ed_tests_llm_16_409_rrrruuuugggg_test_error_code_unknown_encoding = 0;
    }
    #[test]
    fn test_error_code_has_changed() {
        let _rug_st_tests_llm_16_409_rrrruuuugggg_test_error_code_has_changed = 0;
        let rug_fuzz_0 = "test.txt";
        let error = FileError::HasChanged(PathBuf::from(rug_fuzz_0));
        debug_assert_eq!(error.error_code(), 7);
        let _rug_ed_tests_llm_16_409_rrrruuuugggg_test_error_code_has_changed = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_430 {
    use super::*;
    use crate::*;
    use std::path::Path;
    #[test]
    fn test_get_mod_time() {
        let _rug_st_tests_llm_16_430_rrrruuuugggg_test_get_mod_time = 0;
        let rug_fuzz_0 = "test_file.txt";
        let path = Path::new(rug_fuzz_0);
        let result = get_mod_time(path);
        debug_assert!(result.is_some());
        let _rug_ed_tests_llm_16_430_rrrruuuugggg_test_get_mod_time = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_431 {
    use super::*;
    use crate::*;
    #[test]
    fn test_get_permissions() {
        let _rug_st_tests_llm_16_431_rrrruuuugggg_test_get_permissions = 0;
        let rug_fuzz_0 = "/path/to/file.txt";
        let path: PathBuf = rug_fuzz_0.into();
        let result = get_permissions(path);
        debug_assert!(result.is_some());
        debug_assert_eq!(result.unwrap(), 0o777);
        let _rug_ed_tests_llm_16_431_rrrruuuugggg_test_get_permissions = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_436 {
    use super::*;
    use crate::*;
    use std::fs;
    use std::io::Write;
    use std::path::Path;
    use std::io;
    #[test]
    fn test_try_save() -> io::Result<()> {
        let path = Path::new("test_file.txt");
        let text = Rope::from("Hello, world!");
        let encoding = CharacterEncoding::Utf8;
        let file_info = None;
        let result = try_save(path, &text, encoding, file_info);
        assert_eq!(result.is_ok(), true);
        assert_eq!(fs::read_to_string(path).unwrap(), "Hello, world!");
        Ok(())
    }
}
