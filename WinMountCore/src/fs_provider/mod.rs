mod adbfs;
mod archivefs;
pub mod memfs;
mod overlayfs;

use std::{
    collections::HashMap,
    marker::PhantomData,
    num::NonZeroUsize,
    ops::Deref,
    sync::{Arc, RwLock},
    time::SystemTime,
};

use bitflags::bitflags;
use uuid::Uuid;

#[derive(thiserror::Error, Debug)]
pub enum FileSystemError {
    #[error("the path does not exist")]
    ObjectPathNotFound,
    #[error("the requested operation is not implemented")]
    NotImplemented,
    #[error("the file that was specified as a target is a directory and the caller specified that it could be anything but a directory")]
    FileIsADirectory,
    #[error("a requested opened file is not a directory")]
    NotADirectory,
    #[error("the object name is not found")]
    ObjectNameNotFound,
    #[error("the object name already exists")]
    ObjectNameCollision,
    #[error("the directory trying to be deleted is not empty")]
    DirectoryNotEmpty,
    #[error(
        "a process has requested access to an object but has not been granted those access rights"
    )]
    AccessDenied,
    #[error("the file does not exist")]
    NoSuchFile,
    #[error("an attempt has been made to remove a file or directory that cannot be deleted")]
    CannotDelete,
    #[error("the parameter specified in the request is not valid")]
    InvalidParameter,
}

pub type FileSystemResult<T> = Result<T, FileSystemError>;

// TODO: Methods should receive a special path variable that is
//       slash-neutral

#[derive(Debug, Clone, Copy)]
pub enum PathDelimiter {
    Slash,
    BackSlash,
    // TODO: Add option NeutralSlash? and implement as_pattern()
}

impl PathDelimiter {
    pub fn as_char(&self) -> char {
        match self {
            Self::Slash => '/',
            Self::BackSlash => '\\',
        }
    }
}

// Segmented path
#[derive(Debug, Clone, Copy)]
pub struct SegPath<'a> {
    raw_path: &'a str,
    path: &'a str,
    delimiter: PathDelimiter,
}

impl<'a> SegPath<'a> {
    // WARN: Could panic
    pub fn new(path: &'a str, delimiter: PathDelimiter) -> SegPath<'a> {
        if path.contains('\0') {
            panic!("path must not contain nul bytes");
        }
        SegPath {
            raw_path: path,
            path: path.strip_prefix(delimiter.as_char()).unwrap_or(path),
            delimiter,
        }
    }
    pub fn new_truncate(path: &'a str, delimiter: PathDelimiter) -> SegPath<'a> {
        let path = path.split_once('\0').map(|x| x.0).unwrap_or(path);
        SegPath {
            raw_path: path,
            path: path.strip_prefix(delimiter.as_char()).unwrap_or(path),
            delimiter,
        }
    }
    pub unsafe fn new_unchecked(path: &'a str, delimiter: PathDelimiter) -> SegPath<'a> {
        SegPath {
            raw_path: path,
            path: path.strip_prefix(delimiter.as_char()).unwrap_or(path),
            delimiter,
        }
    }
    fn iter(&self) -> SegPathIter<'a> {
        // TODO: Use self.path.split() instead?
        SegPathIter {
            cur_path: "",
            rest_path: self.path,
            delimiter: self.delimiter,
        }
    }
}

pub struct SegPathIter<'a> {
    cur_path: &'a str,
    rest_path: &'a str,
    delimiter: PathDelimiter,
}

impl<'a> Iterator for SegPathIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let delimiter = self.delimiter.as_char();
        if let Some((cur_path, rest_path)) = self.rest_path.split_once(delimiter) {
            self.cur_path = cur_path;
            self.rest_path = rest_path;
            Some(self.cur_path)
        } else {
            self.cur_path = self.rest_path;
            self.rest_path = "";
            (!self.cur_path.is_empty()).then_some(self.cur_path)
        }
    }
}

impl<'a, 'b> IntoIterator for &'b SegPath<'a> {
    type Item = &'a str;
    type IntoIter = SegPathIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// Designed to retrieve paths from FFI
#[derive(Debug, Clone, Copy)]
pub struct U16SegPath<'a> {
    raw_path: &'a widestring::U16CStr,
    path: &'a widestring::U16Str,
    delimiter: PathDelimiter,
}

impl<'a> U16SegPath<'a> {
    pub fn new(path: &'a widestring::U16CStr, delimiter: PathDelimiter) -> U16SegPath<'a> {
        let raw_path = path;
        let path = path
            .as_slice()
            .strip_prefix(&[delimiter.as_char() as u16])
            .unwrap_or(path.as_slice());
        U16SegPath {
            raw_path,
            path: widestring::U16Str::from_slice(path),
            delimiter,
        }
    }
    fn iter(&self) -> U16SegPathIter<'a> {
        U16SegPathIter {
            cur_path: widestring::U16Str::from_slice(&[]),
            rest_path: self.path,
            delimiter: self.delimiter,
        }
    }
}

pub struct U16SegPathIter<'a> {
    cur_path: &'a widestring::U16Str,
    rest_path: &'a widestring::U16Str,
    delimiter: PathDelimiter,
}

impl<'a> Iterator for U16SegPathIter<'a> {
    type Item = &'a widestring::U16Str;

    fn next(&mut self) -> Option<Self::Item> {
        let delimiter = self.delimiter.as_char() as u16;
        let rest_path = self.rest_path.as_slice();
        let split_once = |x: &'a [u16], delimiter| {
            let start = rest_path
                .iter()
                .enumerate()
                .filter_map(|x| (*x.1 == delimiter).then_some(x.0))
                .next();
            if let Some(start) = start {
                let end = start + 1;
                unsafe { Some((x.get_unchecked(..start), x.get_unchecked(end..))) }
            } else {
                None
            }
        };
        if let Some((cur_path, rest_path)) = split_once(rest_path, delimiter) {
            self.cur_path = widestring::U16Str::from_slice(cur_path);
            self.rest_path = widestring::U16Str::from_slice(rest_path);
            Some(self.cur_path)
        } else {
            self.cur_path = self.rest_path;
            self.rest_path = widestring::U16Str::from_slice(&[]);
            (!self.cur_path.is_empty()).then_some(self.cur_path)
        }
    }
}

impl<'a, 'b> IntoIterator for &'b U16SegPath<'a> {
    type Item = &'a widestring::U16Str;
    type IntoIter = U16SegPathIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

struct OwnedSegPath {
    raw_path: String,
    delimiter: PathDelimiter,
}

impl TryFrom<U16SegPath<'_>> for OwnedSegPath {
    type Error = widestring::error::Utf16Error;

    fn try_from(value: U16SegPath) -> Result<Self, Self::Error> {
        let path = value.path.to_string()?;
        Ok(OwnedSegPath {
            raw_path: path,
            delimiter: value.delimiter,
        })
    }
}

impl OwnedSegPath {
    fn from_u16_path_lossy(path: U16SegPath<'_>) -> Self {
        let raw_path = path.raw_path.to_string_lossy();
        OwnedSegPath {
            raw_path,
            delimiter: path.delimiter,
        }
    }

    fn as_non_owned(&self) -> SegPath<'_> {
        unsafe { SegPath::new_unchecked(&self.raw_path, self.delimiter) }
    }
}

pub struct CreateFileInfo<'c> {
    pub context: OwnedFile<'c>,
    pub is_dir: bool,
    pub new_file_created: bool,
}

pub struct FileStatInfo {
    pub index: u64,
    pub size: u64,
    pub is_dir: bool,
    pub attributes: FileAttributes,
    pub creation_time: SystemTime,
    pub last_access_time: SystemTime,
    pub last_write_time: SystemTime,
}

pub trait FilePattern {
    fn check_name(&self, name: &str) -> bool;
}

pub trait WideFilePattern {
    fn check_name(&self, name: &widestring::U16CStr) -> bool;
}

pub struct AcceptAllFilePattern {}
impl AcceptAllFilePattern {
    pub fn new() -> Self {
        Self {}
    }
}
impl FilePattern for AcceptAllFilePattern {
    fn check_name(&self, name: &str) -> bool {
        true
    }
}
impl WideFilePattern for AcceptAllFilePattern {
    fn check_name(&self, name: &widestring::U16CStr) -> bool {
        true
    }
}

pub trait FindFilesDataFiller {
    fn fill_data(&mut self, name: &str, stat: &FileStatInfo) -> Result<(), ()>;
}

pub trait WideFindFilesDataFiller {
    fn fill_data(&mut self, name: &widestring::U16CStr, stat: &FileStatInfo) -> Result<(), ()>;
}

// NOTE: wide functions should be overriden for better performance
// TODO: get_path() should return Option<OwnedSegPath>
pub trait File: Sync {
    fn get_path(&self) -> Option<String> {
        None
    }
    fn read_at(&self, offset: u64, buffer: &mut [u8]) -> FileSystemResult<u64>;
    // If offset is None, data should be appended instead
    fn write_at(
        &self,
        offset: Option<u64>,
        buffer: &[u8],
        constrain_size: bool,
    ) -> FileSystemResult<u64>;
    fn flush_buffers(&self) -> FileSystemResult<()>;
    fn get_stat(&self) -> FileSystemResult<FileStatInfo>;
    fn set_end_of_file(&self, offset: u64) -> FileSystemResult<()>;
    fn set_file_times(
        &self,
        creation_time: SystemTime,
        last_access_time: SystemTime,
        last_write_time: SystemTime,
    ) -> FileSystemResult<()>;
    fn set_delete(&self, delete_on_close: bool) -> FileSystemResult<()>;
    fn move_to(&self, new_path: SegPath, replace_if_exists: bool) -> FileSystemResult<()>;
    fn find_files_with_pattern(
        &self,
        pattern: &dyn FilePattern,
        filler: &mut dyn FindFilesDataFiller,
    ) -> FileSystemResult<()>;
    fn get_wide_path(&self) -> Option<widestring::U16CString> {
        // widestring::U16CString::from_str(self.get_path())
        //     .expect("path must not contain nul bytes")
        self.get_path()
            .map(widestring::U16CString::from_str_truncate)
    }
    fn wide_move_to(&self, new_path: U16SegPath, replace_if_exists: bool) -> FileSystemResult<()> {
        let new_path = OwnedSegPath::from_u16_path_lossy(new_path);
        let new_path = new_path.as_non_owned();
        self.move_to(new_path, replace_if_exists)
    }
    fn wide_find_files_with_pattern(
        &self,
        pattern: &dyn WideFilePattern,
        filler: &mut dyn WideFindFilesDataFiller,
    ) -> FileSystemResult<()> {
        struct ToWideFilePatternWrapper<'a> {
            pattern: &'a dyn WideFilePattern,
        }
        impl FilePattern for ToWideFilePatternWrapper<'_> {
            fn check_name(&self, name: &str) -> bool {
                let name = widestring::U16CString::from_str_truncate(name);
                self.pattern.check_name(&name)
            }
        }
        struct ToWideFindFilesDataFillerWrapper<'a> {
            filler: &'a mut dyn WideFindFilesDataFiller,
        }
        impl FindFilesDataFiller for ToWideFindFilesDataFillerWrapper<'_> {
            fn fill_data(&mut self, name: &str, stat: &FileStatInfo) -> Result<(), ()> {
                let name = widestring::U16CString::from_str_truncate(name);
                self.filler.fill_data(&name, stat)
            }
        }
        let pattern = ToWideFilePatternWrapper { pattern };
        let mut filler = ToWideFindFilesDataFillerWrapper { filler };
        self.find_files_with_pattern(&pattern, &mut filler)
    }
}

pub type OwnedFile<'c> = Box<dyn File + 'c>;

pub struct FileSystemSpaceInfo {
    /// Total number of bytes that are available to the calling user.
    pub bytes_count: u64,

    /// Total number of free bytes on the disk.
    pub free_bytes_count: u64,

    /// Total number of free bytes that are available to the calling user.
    pub available_bytes_count: u64,
}

pub trait FileSystemHandler: Send + Sync {
    fn create_file(
        &self,
        filename: SegPath,
        desired_access: FileDesiredAccess,
        file_attributes: FileAttributes,
        share_access: FileShareAccess,
        create_disposition: FileCreateDisposition,
        create_options: FileCreateOptions,
    ) -> FileSystemResult<CreateFileInfo<'_>>;
    fn wide_create_file(
        &self,
        filename: U16SegPath,
        desired_access: FileDesiredAccess,
        file_attributes: FileAttributes,
        share_access: FileShareAccess,
        create_disposition: FileCreateDisposition,
        create_options: FileCreateOptions,
    ) -> FileSystemResult<CreateFileInfo<'_>> {
        let filename = OwnedSegPath::from_u16_path_lossy(filename);
        let filename = filename.as_non_owned();
        self.create_file(
            filename,
            desired_access,
            file_attributes,
            share_access,
            create_disposition,
            create_options,
        )
    }
    // fn move_file(
    //     &self,
    //     file: either::Either<&mut OwnedFile<'_>, &str>,
    //     dest_path: &str
    // ) -> FileSystemResult<()>;
    // fn wide_move_file(
    //     &self,
    //     file: either::Either<&mut OwnedFile<'_>, U16SegPath>,
    //     dest_path: U16SegPath
    // ) -> FileSystemResult<()>;
    // fn delete_file(&self, path: SegPath) -> FileSystemResult<()>;
    // fn wide_delete_file(&self, path: U16SegPath) -> FileSystemResult<()>;
    fn get_fs_free_space(&self) -> FileSystemResult<FileSystemSpaceInfo>;
    // TODO: Characteristics: read-only, case-sensitive, ...
    fn get_fs_characteristics(&self) -> FileSystemResult<FileSystemCharacteristics>;
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct FileDesiredAccess: u32 {
        const Read = 0x80000000;
        const Write = 0x40000000;
        const Execute = 0x20000000;
        const Full = 0x10000000;
        const ReadWrite = Self::Read.bits() | Self::Write.bits();
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct FileShareAccess: u32 {
        const Read = 0x1;
        const Write = 0x2;
        const Delete = 0x4;
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct FileAttributes: u32 {
        const Readonly = 0x1;
        const Hidden = 0x2;
        const DirectoryFile = 0x4;
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct FileCreateOptions: u32 {
        const DeleteOnClose = 0x1;
        const DirectoryFile = 0x2;
        const NonDirectoryFile = 0x4;
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct FileSystemCharacteristics : u32 {
        const ReadOnly = 0x1;
        const CaseSensitive = 0x2;
    }
}

impl FileAttributes {
    fn is_normal(&self) -> bool {
        self.is_empty()
    }
}

#[repr(u32)]
pub enum FileCreateDisposition {
    CreateNew = 1,
    CreateAlways = 2,
    OpenExisting = 3,
    OpenAlways = 4,
    TruncateExisting = 5,
}

pub struct FileSystem {
    id: Uuid,
    kind_id: Uuid,
    name: String,
    handler: Arc<dyn FileSystemHandler>,
}

#[derive(Debug, thiserror::Error)]
pub enum FileSystemCreationError {
    #[error("the filesystem is not found")]
    NotFound,
    #[error("the filesystem depends on itself in some way, preventing the creation")]
    CyclicDependency,
    #[error("the filesystem configuration is invalid")]
    InvalidFileSystem,
    #[error("other error")]
    Other(#[from] anyhow::Error),
}

pub trait FileSystemCreationContext {
    fn get_or_run_fs(
        &mut self,
        id: &Uuid,
        prefix_path: &str,
    ) -> Result<Arc<dyn FileSystemHandler>, FileSystemCreationError>;
}

pub trait FsProvider: Send {
    fn get_id(&self) -> Uuid;
    fn get_name(&self) -> &'static str;
    fn construct(
        &self,
        config: serde_json::Value,
        ctx: &mut dyn FileSystemCreationContext,
    ) -> Result<Arc<dyn FileSystemHandler>, FileSystemCreationError>;
}

pub fn init_fs_providers(
    mut register_fn: impl FnMut(Uuid, Box<dyn FsProvider>) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut reg = |p: Box<dyn FsProvider>| register_fn(p.get_id(), p);
    reg(Box::new(memfs::MemFsProvider::new()))?;
    Ok(())
}

pub fn uninit_fs_providers() {
    // TODO: uninit_fs_providers
}
