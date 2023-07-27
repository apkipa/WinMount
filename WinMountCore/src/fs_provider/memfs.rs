use std::{
    borrow::Borrow,
    collections::BTreeMap,
    ops::Deref,
    sync::{Arc, RwLock},
    time::SystemTime,
};

use uuid::{uuid, Uuid};
use widestring::{U16CStr, U16CString};

use super::{FileCreateDisposition, FileSystemError, FileSystemHandler};

// TODO: Add option to use AWE for allocating non-paged memory

#[derive(Debug, Clone, Copy)]
struct FileStat {
    attributes: super::FileAttributes,
    creation_time: SystemTime,
    last_access_time: SystemTime,
    last_write_time: SystemTime,
    delete_on_close: bool,
}

enum Entry {
    File(Arc<RwLock<FileEntry>>),
    Folder(Arc<RwLock<FolderEntry>>),
}

impl Entry {
    fn is_dir(&self) -> bool {
        match self {
            Self::File(_) => false,
            Self::Folder(_) => true,
        }
    }

    fn get_stat(&self) -> FileStat {
        match self {
            Self::File(f) => f.read().unwrap().stat,
            Self::Folder(f) => f.read().unwrap().stat,
        }
    }
    fn set_stat(&self, stat: &FileStat) {
        match self {
            Self::File(f) => f.write().unwrap().stat = *stat,
            Self::Folder(f) => f.write().unwrap().stat = *stat,
        }
    }

    fn modify_stat(&self, mod_fn: impl FnOnce(&mut FileStat)) {
        match self {
            Self::File(f) => mod_fn(&mut f.write().unwrap().stat),
            Self::Folder(f) => mod_fn(&mut f.write().unwrap().stat),
        }
    }

    fn get_file_stat_info(&self) -> super::FileStatInfo {
        match self {
            Entry::File(f) => {
                let index = Arc::as_ptr(f) as _;
                let f = f.read().unwrap();
                super::FileStatInfo {
                    index,
                    size: f.data.len() as _,
                    attributes: super::FileAttributes::empty(),
                    is_dir: false,
                    creation_time: f.stat.creation_time,
                    last_access_time: f.stat.last_access_time,
                    last_write_time: f.stat.last_write_time,
                }
            }
            Entry::Folder(f) => {
                let index = Arc::as_ptr(f) as _;
                let f = f.read().unwrap();
                super::FileStatInfo {
                    index,
                    size: 0,
                    attributes: super::FileAttributes::empty(),
                    is_dir: true,
                    creation_time: f.stat.creation_time,
                    last_access_time: f.stat.last_access_time,
                    last_write_time: f.stat.last_write_time,
                }
            }
        }
    }
}

impl Clone for Entry {
    fn clone(&self) -> Self {
        match self {
            Self::File(v) => Self::File(Arc::clone(v)),
            Self::Folder(v) => Self::Folder(Arc::clone(v)),
        }
    }
}

struct CaselessStr(str);
impl PartialEq for CaselessStr {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}
impl Eq for CaselessStr {}
impl PartialOrd for CaselessStr {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for CaselessStr {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // We convert everything to lowercase for comparison
        for (x, y) in self.0.bytes().zip(other.0.bytes()) {
            let (x, y) = (x.to_ascii_lowercase(), y.to_ascii_lowercase());
            let r = x.cmp(&y);
            if r != std::cmp::Ordering::Equal {
                return r;
            }
        }
        self.0.len().cmp(&other.0.len())
    }
}
impl CaselessStr {
    fn new(value: &str) -> &Self {
        unsafe { std::mem::transmute(value) }
    }
    fn as_str(&self) -> &str {
        &self.0
    }
}

struct CaselessString(String);
impl PartialEq for CaselessString {
    fn eq(&self, other: &Self) -> bool {
        self.deref().eq(other)
    }
}
impl Eq for CaselessString {}
impl PartialOrd for CaselessString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for CaselessString {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.deref().cmp(other)
    }
}
impl Deref for CaselessString {
    type Target = CaselessStr;

    fn deref(&self) -> &Self::Target {
        CaselessStr::new(&self.0)
    }
}
impl From<String> for CaselessString {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
impl CaselessString {
    fn new(value: String) -> Self {
        Self(value)
    }
}

struct CaselessU16CString(U16CString);
impl PartialEq for CaselessU16CString {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && std::iter::zip(self.0.as_slice(), other.0.as_slice()).all(|(&x, &y)| {
                match (u8::try_from(x), u8::try_from(y)) {
                    (Ok(x), Ok(y)) => x.eq_ignore_ascii_case(&y),
                    _ => false,
                }
            })
    }
}
impl Eq for CaselessU16CString {}
impl PartialOrd for CaselessU16CString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for CaselessU16CString {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // TODO: Fix cmp (does not correctly take case into consideration)
        self.0.cmp(&other.0)
    }
}
impl From<U16CString> for CaselessU16CString {
    fn from(value: U16CString) -> Self {
        Self::new(value)
    }
}
impl CaselessU16CString {
    fn new(value: U16CString) -> Self {
        Self(value)
    }
}

// struct FileName {
//     name: String,
//     u16_name: U16CString,
// }
struct FileName {
    name: CaselessString,
    u16_name: CaselessU16CString,
}

impl PartialEq for FileName {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for FileName {}

impl Ord for FileName {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}
impl PartialOrd for FileName {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Borrow<CaselessStr> for FileName {
    fn borrow(&self) -> &CaselessStr {
        &self.name
    }
}
// impl Borrow<widestring::U16CStr> for FileName {
//     fn borrow(&self) -> &widestring::U16CStr {
//         &self.u16_name
//     }
// }

impl From<&str> for FileName {
    fn from(value: &str) -> Self {
        FileName {
            name: value.to_owned().into(),
            u16_name: U16CString::from_str_truncate(value).into(),
        }
    }
}

impl From<&U16CStr> for FileName {
    fn from(value: &U16CStr) -> Self {
        FileName {
            name: value.to_string_lossy().into(),
            u16_name: value.to_owned().into(),
        }
    }
}

struct FileEntry {
    stat: FileStat,
    parent: std::sync::Weak<RwLock<FolderEntry>>,
    data: Vec<u8>,
}

struct FolderEntry {
    stat: FileStat,
    parent: std::sync::Weak<RwLock<FolderEntry>>,
    children: BTreeMap<FileName, Entry>,
}

struct MemFsHandler {
    root_folder: Arc<RwLock<FolderEntry>>,
}

struct MemFsFile<'h> {
    fs_handler: &'h MemFsHandler,
    obj: Entry,
    // TODO: This indicates ownership of delete_on_close in the file instance?
    delete_on_close: bool,
}

impl MemFsHandler {
    fn resolve_path<'s>(
        &self,
        path: super::SegPath<'s>,
    ) -> super::FileSystemResult<(Option<Arc<RwLock<FolderEntry>>>, &'s str)> {
        let mut parent: Option<Arc<RwLock<FolderEntry>>> = None;
        let mut cur_dir: Arc<RwLock<FolderEntry>> = Arc::clone(&self.root_folder);
        let mut it = path.into_iter().peekable();
        let mut non_empty = false;
        let mut filename = "";
        while let Some(path) = it.next() {
            non_empty = true;
            if let None = it.peek() {
                // Find in last level
                filename = path;
                break;
            }
            // Find the next folder
            let next_dir = if let Some(Entry::Folder(folder)) =
                cur_dir.read().unwrap().children.get(CaselessStr::new(path))
            {
                Arc::clone(folder)
            } else {
                return Err(super::FileSystemError::ObjectPathNotFound);
            };
            cur_dir = next_dir;
        }
        if non_empty {
            parent = Some(cur_dir);
        }
        Ok((parent, filename))
    }
}

pub const MEMFS_ID: Uuid = uuid!("A93FB2C4-1A4A-4510-9826-7B72A5AFDE45");

impl FileSystemHandler for MemFsHandler {
    fn create_file(
        &self,
        filename: super::SegPath,
        desired_access: super::FileDesiredAccess,
        file_attributes: super::FileAttributes,
        share_access: super::FileShareAccess,
        create_disposition: FileCreateDisposition,
        create_options: super::FileCreateOptions,
    ) -> super::FileSystemResult<super::CreateFileInfo<'_>> {
        use FileCreateDisposition::*;

        let expects_dir = create_options.contains(super::FileCreateOptions::DirectoryFile);
        let expects_nondir = create_options.contains(super::FileCreateOptions::NonDirectoryFile);
        let delete_on_close = create_options.contains(super::FileCreateOptions::DeleteOnClose);

        // log::trace!("CreateFile, filename = `{}`, delete = {delete_on_close}", filename.raw_path);

        let (parent, filename) = self.resolve_path(filename)?;

        // Behaviour table: https://stackoverflow.com/a/14469641
        let mut is_dir = expects_dir;
        let mut new_file_created = false;

        let entry = if let Some(parent) = parent {
            let mut handle_exists = |child: &Entry| -> super::FileSystemResult<Entry> {
                Ok(match child {
                    Entry::File(file) => {
                        if expects_dir {
                            return Err(FileSystemError::NotADirectory);
                        }

                        match create_disposition {
                            CreateAlways | TruncateExisting => {
                                file.write().unwrap().data.clear();
                                Entry::File(Arc::clone(file))
                            }
                            OpenAlways | OpenExisting => Entry::File(Arc::clone(file)),
                            CreateNew => return Err(FileSystemError::ObjectNameCollision),
                        }
                    }
                    Entry::Folder(folder) => {
                        if expects_nondir {
                            return Err(FileSystemError::FileIsADirectory);
                        }

                        is_dir = true;

                        // TODO: Check children non-empty if is deleting

                        match create_disposition {
                            CreateAlways | OpenAlways | OpenExisting | TruncateExisting => {
                                Entry::Folder(Arc::clone(folder))
                            }
                            CreateNew => return Err(FileSystemError::ObjectNameCollision),
                        }
                    }
                })
            };
            loop {
                if let Some(child) = parent
                    .read()
                    .unwrap()
                    .children
                    .get(CaselessStr::new(filename))
                {
                    // Object exists
                    break handle_exists(child)?;
                }
                // Object does not exist
                match create_disposition {
                    CreateAlways | CreateNew | OpenAlways => {
                        // Re-lock (upgrade to write), then test again
                        use std::collections::btree_map::Entry::*;
                        break match parent.write().unwrap().children.entry(filename.into()) {
                            Occupied(e) => handle_exists(e.get())?,
                            Vacant(e) => {
                                new_file_created = true;
                                let cur_t = SystemTime::now();
                                let file_stat = FileStat {
                                    attributes: super::FileAttributes::empty(),
                                    creation_time: cur_t,
                                    last_access_time: cur_t,
                                    last_write_time: cur_t,
                                    delete_on_close,
                                };
                                let entry = if is_dir {
                                    Entry::Folder(Arc::new(RwLock::new(FolderEntry {
                                        stat: file_stat,
                                        parent: Arc::downgrade(&parent),
                                        children: BTreeMap::new(),
                                    })))
                                } else {
                                    Entry::File(Arc::new(RwLock::new(FileEntry {
                                        stat: file_stat,
                                        parent: Arc::downgrade(&parent),
                                        data: Vec::new(),
                                    })))
                                };
                                e.insert(entry).clone()
                            }
                        };
                    }
                    _ => return Err(FileSystemError::ObjectNameNotFound),
                }
            }
        } else {
            // Root folder
            if expects_nondir {
                return Err(FileSystemError::FileIsADirectory);
            }
            if delete_on_close {
                return Err(FileSystemError::AccessDenied);
            }

            is_dir = true;

            match create_disposition {
                CreateAlways | OpenAlways | OpenExisting | TruncateExisting => {
                    Entry::Folder(Arc::clone(&self.root_folder))
                }
                CreateNew => return Err(FileSystemError::ObjectNameCollision),
            }
        };

        // TODO: Change delete logic
        if delete_on_close {
            match &entry {
                Entry::Folder(f) => {
                    f.write().unwrap().stat.delete_on_close = true;
                }
                Entry::File(f) => {
                    f.write().unwrap().stat.delete_on_close = true;
                }
            }
        }

        Ok(super::CreateFileInfo {
            context: Box::new(MemFsFile {
                fs_handler: self,
                obj: entry,
                delete_on_close,
            }),
            is_dir,
            new_file_created,
        })
    }
    fn get_fs_free_space(&self) -> super::FileSystemResult<super::FileSystemSpaceInfo> {
        let total = 1024 * 1024 * 1024 * 8;
        let used = 1024 * 1024 * 1024 * 1;
        Ok(super::FileSystemSpaceInfo {
            bytes_count: total,
            free_bytes_count: total - used,
            available_bytes_count: total - used,
        })
    }
    fn get_fs_characteristics(&self) -> super::FileSystemResult<super::FileSystemCharacteristics> {
        Ok(super::FileSystemCharacteristics::empty())
    }
}

impl MemFsHandler {
    fn new() -> Self {
        let ts_now = SystemTime::now();
        Self {
            root_folder: Arc::new_cyclic(|x| {
                RwLock::new(FolderEntry {
                    stat: FileStat {
                        attributes: super::FileAttributes::DirectoryFile,
                        creation_time: ts_now,
                        last_access_time: ts_now,
                        last_write_time: ts_now,
                        delete_on_close: false,
                    },
                    parent: x.clone(),
                    children: BTreeMap::new(),
                })
            }),
        }
    }
}

impl MemFsFile<'_> {
    fn remove_from_parent(&self, obj_ptr: usize, parent: &mut FolderEntry) -> Option<Entry> {
        // TODO: Use references to names to improve delete performance and avoid extra copies
        let child_ptr = obj_ptr;
        let mut is_removed = false;
        parent.children.retain(|k, v| {
            let in_map_ptr: usize = match v {
                Entry::Folder(f) => Arc::as_ptr(f) as _,
                Entry::File(f) => Arc::as_ptr(f) as _,
            };
            if in_map_ptr == child_ptr {
                is_removed = true;
            }
            in_map_ptr != child_ptr
        });
        if is_removed {
            Some(self.obj.clone())
        } else {
            None
        }
    }
}

impl super::File for MemFsFile<'_> {
    fn read_at(&self, offset: u64, buffer: &mut [u8]) -> super::FileSystemResult<u64> {
        // log::trace!("Read at offset = {offset}, size = {}", buffer.len());
        match &self.obj {
            Entry::Folder(_) => Err(FileSystemError::FileIsADirectory),
            Entry::File(f) => {
                let f = f.read().unwrap();
                if offset >= f.data.len() as _ {
                    Ok(0)
                } else {
                    let real_len = buffer.len().min(f.data.len() - offset as usize);
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            f.data.as_ptr().add(offset as _),
                            buffer.as_mut_ptr(),
                            real_len,
                        );
                    }
                    Ok(real_len as _)
                }
            }
        }
    }
    fn write_at(
        &self,
        offset: Option<u64>,
        buffer: &[u8],
        constrain_size: bool,
    ) -> super::FileSystemResult<u64> {
        // log::trace!("Write at offset = {offset:?}, size = {}", buffer.len());
        match &self.obj {
            Entry::Folder(_) => Err(FileSystemError::FileIsADirectory),
            Entry::File(f) => {
                let mut f = f.write().unwrap();
                let offset = offset.map(|x| x as _).unwrap_or(f.data.len());
                if constrain_size {
                    if offset >= f.data.len() as _ {
                        Ok(0)
                    } else {
                        let real_len = buffer.len().min(f.data.len() - offset as usize);
                        unsafe {
                            std::ptr::copy_nonoverlapping(
                                buffer.as_ptr(),
                                f.data.as_mut_ptr().add(offset as _),
                                real_len,
                            );
                        }
                        Ok(real_len as _)
                    }
                } else {
                    let final_size = offset as usize + buffer.len();
                    let orig_len = f.data.len();
                    let mut should_update_len = false;
                    if final_size > orig_len {
                        f.data.reserve(final_size - orig_len);
                        should_update_len = true;
                    }
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            buffer.as_ptr(),
                            f.data.as_mut_ptr().add(offset as _),
                            buffer.len(),
                        );
                        if should_update_len {
                            f.data.set_len(final_size);
                        }
                    }
                    Ok(buffer.len() as _)
                }
            }
        }
    }
    fn flush_buffers(&self) -> super::FileSystemResult<()> {
        log::trace!("Flush buffers");
        Ok(())
    }
    fn get_stat(&self) -> super::FileSystemResult<super::FileStatInfo> {
        // log::trace!("Get stat");
        Ok(self.obj.get_file_stat_info())
    }
    fn set_end_of_file(&self, offset: u64) -> super::FileSystemResult<()> {
        // log::trace!("Set EOF at offset = {offset}");
        match &self.obj {
            Entry::Folder(_) => Err(FileSystemError::FileIsADirectory),
            Entry::File(f) => {
                let mut f = f.write().unwrap();
                f.data.resize(offset as _, 0);
                Ok(())
            }
        }
    }
    fn set_file_times(
        &self,
        creation_time: SystemTime,
        last_access_time: SystemTime,
        last_write_time: SystemTime,
    ) -> super::FileSystemResult<()> {
        // log::trace!("Set file times");
        self.obj.modify_stat(|stat| {
            let zero_t = unsafe { std::mem::zeroed() };
            if creation_time != zero_t {
                stat.creation_time = creation_time;
            }
            if last_access_time != zero_t {
                stat.last_access_time = last_access_time;
            }
            if last_write_time != zero_t {
                stat.last_write_time = last_write_time;
            }
        });
        Ok(())
    }
    fn set_delete(&self, delete_on_close: bool) -> super::FileSystemResult<()> {
        log::trace!("Set delete: {delete_on_close}");
        // TODO: Use correct NTFS delete semantics
        match &self.obj {
            Entry::File(f) => {
                f.write().unwrap().stat.delete_on_close = delete_on_close;
            },
            Entry::Folder(f) => {
                let mut f = f.write().unwrap();
                if delete_on_close && !f.children.is_empty() {
                    return Err(FileSystemError::DirectoryNotEmpty);
                }
                f.stat.delete_on_close = delete_on_close;
            }
        }
        // self.delete_on_close = delete_on_close;
        Ok(())
    }
    fn move_to(
        &self,
        new_path: super::SegPath,
        replace_if_exists: bool,
    ) -> super::FileSystemResult<()> {
        log::trace!(
            "Move to: {}, replace: {replace_if_exists}",
            new_path.raw_path
        );

        let (parent, filename) = self.fs_handler.resolve_path(new_path)?;
        if let Some(new_parent) = parent {
            let handle_fn = |child_ptr, old_parent| {
                if Arc::ptr_eq(&old_parent, &new_parent) {
                    // Movement inside the same folder
                    let mut parent = new_parent.write().unwrap();
                    if !replace_if_exists
                        && parent.children.contains_key(CaselessStr::new(filename))
                    {
                        return Err(FileSystemError::ObjectNameCollision);
                    }
                    let entry = self
                        .remove_from_parent(child_ptr, &mut parent)
                        .ok_or(FileSystemError::AccessDenied)?;
                    parent.children.insert(filename.into(), entry);
                } else {
                    // Movement between different folders
                    let mut old_parent = old_parent.write().unwrap();
                    let mut new_parent = new_parent.write().unwrap();
                    if !replace_if_exists
                        && new_parent.children.contains_key(CaselessStr::new(filename))
                    {
                        return Err(FileSystemError::ObjectNameCollision);
                    }
                    let entry = self
                        .remove_from_parent(child_ptr, &mut old_parent)
                        .ok_or(FileSystemError::AccessDenied)?;
                    new_parent.children.insert(filename.into(), entry);
                }
                Ok(())
            };
            match &self.obj {
                Entry::Folder(f) => {
                    let child_ptr = Arc::as_ptr(f) as _;
                    let mut f = f.write().unwrap();
                    handle_fn(
                        child_ptr,
                        f.parent.upgrade().ok_or(FileSystemError::AccessDenied)?,
                    )?;
                    // Reset parent
                    f.parent = Arc::downgrade(&new_parent);
                }
                Entry::File(f) => {
                    let child_ptr = Arc::as_ptr(f) as _;
                    let mut f = f.write().unwrap();
                    handle_fn(
                        child_ptr,
                        f.parent.upgrade().ok_or(FileSystemError::AccessDenied)?,
                    )?;
                    // Reset parent
                    f.parent = Arc::downgrade(&new_parent);
                }
            };
            Ok(())
        } else {
            Err(FileSystemError::AccessDenied)
        }
    }
    fn find_files_with_pattern(
        &self,
        pattern: &dyn super::FilePattern,
        filler: &dyn super::FindFilesDataFiller,
    ) -> super::FileSystemResult<()> {
        // log::trace!("Find files with pattern");
        match &self.obj {
            Entry::File(_) => Err(FileSystemError::NotADirectory),
            Entry::Folder(f) => {
                let f = f.read().unwrap();
                for (name, entry) in f
                    .children
                    .iter()
                    .filter(|x| pattern.check_name(x.0.name.as_str()))
                {
                    if filler
                        .fill_data(name.name.as_str(), &entry.get_file_stat_info())
                        .is_err()
                    {
                        log::warn!("Failed to fill object data");
                    }
                }
                Ok(())
            }
        }
    }
}

impl Drop for MemFsFile<'_> {
    fn drop(&mut self) {
        let (child_ptr, parent): (usize, _) = match &self.obj {
            Entry::Folder(f) => {
                let data = f.read().unwrap();
                if !data.stat.delete_on_close {
                    return;
                }
                if let Some(parent) = data.parent.upgrade() {
                    (Arc::as_ptr(f) as _, parent)
                } else {
                    return;
                }
            }
            Entry::File(f) => {
                let data = f.read().unwrap();
                if !data.stat.delete_on_close {
                    return;
                }
                if let Some(parent) = data.parent.upgrade() {
                    (Arc::as_ptr(f) as _, parent)
                } else {
                    return;
                }
            }
        };
        // TODO: Optimize lookup performance
        parent.write().unwrap().children.retain(|k, v| {
            let in_map_ptr: usize = match v {
                Entry::Folder(f) => Arc::as_ptr(f) as _,
                Entry::File(f) => Arc::as_ptr(f) as _,
            };
            if in_map_ptr == child_ptr {
                log::trace!("Removing file `{}`...", k.name.as_str());
            }
            in_map_ptr != child_ptr
        });
    }
}

pub struct MemFsProvider {}
impl super::FsProvider for MemFsProvider {
    fn get_id(&self) -> Uuid {
        MEMFS_ID
    }
    fn get_name(&self) -> &'static str {
        "memfs"
    }
    fn construct(
        &self,
        config: serde_json::Value,
        ctx: &mut dyn super::FileSystemCreationContext,
    ) -> Result<Arc<dyn FileSystemHandler>, super::FileSystemCreationError> {
        Ok(Arc::new(MemFsHandler::new()))
    }
}

impl MemFsProvider {
    pub fn new() -> Self {
        Self {}
    }
}
