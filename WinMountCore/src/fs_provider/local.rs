// local: Provides access to local filesystem (C:, etc.)
// NOTE: Filesystem local is designed to be stateless and used as a global variable

use std::{mem::MaybeUninit, sync::Arc};

use uuid::{uuid, Uuid};
use windows::{
    core::PCWSTR,
    Wdk::{
        Foundation::{NtClose, OBJECT_ATTRIBUTES},
        Storage::FileSystem::{
            NtCreateFile, NtFlushBuffersFileEx, NtQueryDirectoryFile, NtQueryInformationFile,
            NtReadFile, NtSetInformationFile, NtWriteFile, RtlInitUnicodeStringEx,
            FILE_ALL_INFORMATION, FILE_BASIC_INFORMATION, FILE_CREATE, FILE_DELETE_ON_CLOSE,
            FILE_DIRECTORY_FILE, FILE_DIRECTORY_INFORMATION, FILE_DISPOSITION_INFORMATION,
            FILE_INTERNAL_INFORMATION, FILE_NON_DIRECTORY_FILE, FILE_OPEN, FILE_OPEN_IF,
            FILE_OVERWRITE, FILE_OVERWRITE_IF, FILE_STANDARD_INFORMATION,
        },
        System::SystemServices::{
            FILE_ATTRIBUTE_TAG_INFORMATION, FILE_END_OF_FILE_INFORMATION, FILE_WRITE_TO_END_OF_FILE,
        },
    },
    Win32::{
        Foundation::{
            CloseHandle, BOOLEAN, GENERIC_ALL, GENERIC_EXECUTE, GENERIC_READ, GENERIC_WRITE,
            HANDLE, NTSTATUS, STATUS_ACCESS_DENIED, STATUS_END_OF_FILE, STATUS_FILE_IS_A_DIRECTORY,
            STATUS_NOT_A_DIRECTORY, STATUS_NO_MORE_FILES, STATUS_OBJECT_NAME_COLLISION,
            STATUS_OBJECT_NAME_INVALID, STATUS_OBJECT_NAME_NOT_FOUND, STATUS_OBJECT_PATH_NOT_FOUND,
            UNICODE_STRING, WAIT_OBJECT_0,
        },
        Storage::FileSystem::{
            DELETE, FILE_ACCESS_RIGHTS, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_HIDDEN,
            FILE_ATTRIBUTE_READONLY, FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
            FILE_SHARE_READ, FILE_SHARE_WRITE,
        },
        System::{
            Threading::{CreateEventW, WaitForSingleObject, INFINITE},
            WindowsProgramming::{FileDirectoryInformation, FILE_CREATED, FILE_INFORMATION_CLASS},
            IO::IO_STATUS_BLOCK,
        },
    },
};

use super::{
    FileAttributes, FileCreateDisposition, FileCreateOptions, FileDesiredAccess, FileShareAccess,
    FileSystemError,
};

pub const LOCALFS_ID: Uuid = uuid!("1734A44B-605D-43F6-8BBE-E92BD3336D69");

// TODO: Correctly map some NTSTATUS`es to FileSystemError`s

const FileBasicInformation: FILE_INFORMATION_CLASS = FILE_INFORMATION_CLASS(4);
const FileStandardInformation: FILE_INFORMATION_CLASS = FILE_INFORMATION_CLASS(5);
const FileInternalInformation: FILE_INFORMATION_CLASS = FILE_INFORMATION_CLASS(6);
const FileRenameInformation: FILE_INFORMATION_CLASS = FILE_INFORMATION_CLASS(10);
const FileDispositionInformation: FILE_INFORMATION_CLASS = FILE_INFORMATION_CLASS(13);
const FileAllInformation: FILE_INFORMATION_CLASS = FILE_INFORMATION_CLASS(18);
const FileEndOfFileInformation: FILE_INFORMATION_CLASS = FILE_INFORMATION_CLASS(20);
const FileAttributeTagInformation: FILE_INFORMATION_CLASS = FILE_INFORMATION_CLASS(35);

fn nt_file_attributes_to_local(nt_file_attr: u32) -> FileAttributes {
    let mut x = FileAttributes::empty();
    if (nt_file_attr & FILE_ATTRIBUTE_READONLY.0) != 0 {
        x |= FileAttributes::Readonly;
    }
    if (nt_file_attr & FILE_ATTRIBUTE_HIDDEN.0) != 0 {
        x |= FileAttributes::Hidden;
    }
    if (nt_file_attr & FILE_ATTRIBUTE_DIRECTORY.0) != 0 {
        x |= FileAttributes::DirectoryFile;
    }
    x
}

struct LocalFsHandler {}

impl super::FileSystemHandler for LocalFsHandler {
    fn create_file(
        &self,
        filename: super::SegPath,
        desired_access: FileDesiredAccess,
        file_attributes: FileAttributes,
        share_access: FileShareAccess,
        create_disposition: FileCreateDisposition,
        create_options: FileCreateOptions,
    ) -> super::FileSystemResult<super::CreateFileInfo<'_>> {
        use FileCreateDisposition::*;

        // TODO: Maybe we should sanitize input path to prevent access to
        //       unwanted objects?
        // TODO: Handle root directory (with GetLogicalDrives)?

        // log::debug!("Opening `{}`...", filename.get_path());

        // Prefix for NT Namespace
        const PREFIX: &str = r"\??\";
        // const PREFIX: &str = r"\??\C:\";
        // TODO: Directly initialize UNICODE_STRING instead of appending '\0'
        let filename_buf: Vec<_> = PREFIX
            .encode_utf16()
            .chain(filename.get_path().encode_utf16().map(|x| {
                if x == '/' as u16 {
                    '\\' as _
                } else {
                    x
                }
            }))
            .chain(std::iter::once('\0' as _))
            .collect();
        let filename = filename_buf.as_slice();
        // SAFETY: filename is kept alive by shadowing
        let filename = unsafe {
            let filename = PCWSTR::from_raw(filename.as_ptr());
            let mut us = MaybeUninit::<UNICODE_STRING>::uninit();
            RtlInitUnicodeStringEx(us.as_mut_ptr(), filename)
                .map_err(|e| FileSystemError::Other(e.into()))?;
            us.assume_init()
        };
        let object_attributes = OBJECT_ATTRIBUTES {
            Length: std::mem::size_of::<OBJECT_ATTRIBUTES>() as _,
            RootDirectory: Default::default(),
            ObjectName: &filename,
            Attributes: 0,
            SecurityDescriptor: std::ptr::null(),
            SecurityQualityOfService: std::ptr::null(),
        };
        let desired_access = {
            let mut nt_desired_access = FILE_READ_ATTRIBUTES;
            if desired_access.contains(FileDesiredAccess::Delete) {
                nt_desired_access |= DELETE;
            }
            if desired_access.contains(FileDesiredAccess::Read) {
                nt_desired_access |= FILE_ACCESS_RIGHTS(GENERIC_READ.0);
            }
            if desired_access.contains(FileDesiredAccess::Write) {
                nt_desired_access |= FILE_ACCESS_RIGHTS(GENERIC_WRITE.0);
            }
            if desired_access.contains(FileDesiredAccess::Execute) {
                nt_desired_access |= FILE_ACCESS_RIGHTS(GENERIC_EXECUTE.0);
            }
            if desired_access.contains(FileDesiredAccess::Full) {
                nt_desired_access |= FILE_ACCESS_RIGHTS(GENERIC_ALL.0);
            }
            if desired_access.contains(FileDesiredAccess::ListDirectory) {
                nt_desired_access |= FILE_LIST_DIRECTORY;
            }
            nt_desired_access
        };
        let share_access = {
            let mut nt_share_access = Default::default();
            if share_access.contains(FileShareAccess::Read) {
                nt_share_access |= FILE_SHARE_READ;
            }
            if share_access.contains(FileShareAccess::Write) {
                nt_share_access |= FILE_SHARE_WRITE;
            }
            if share_access.contains(FileShareAccess::Delete) {
                nt_share_access |= FILE_SHARE_DELETE;
            }
            nt_share_access
        };
        let create_disposition = match create_disposition {
            CreateNew => FILE_CREATE,
            CreateAlways => FILE_OVERWRITE_IF,
            OpenExisting => FILE_OPEN,
            OpenAlways => FILE_OPEN_IF,
            TruncateExisting => FILE_OVERWRITE,
            _ => return Err(FileSystemError::InvalidParameter),
        };
        let file_attributes = {
            let mut nt_file_attributes = Default::default();
            if file_attributes.contains(FileAttributes::Readonly) {
                nt_file_attributes |= FILE_ATTRIBUTE_READONLY;
            }
            if file_attributes.contains(FileAttributes::Hidden) {
                nt_file_attributes |= FILE_ATTRIBUTE_HIDDEN;
            }
            if file_attributes.contains(FileAttributes::DirectoryFile) {
                // TODO: Handle FileAttributes::DirectoryFile?
                // return Err(FileSystemError::InvalidParameter);
            }
            nt_file_attributes
        };
        let create_options = {
            let mut nt_create_options = Default::default();
            if create_options.contains(FileCreateOptions::DeleteOnClose) {
                nt_create_options |= FILE_DELETE_ON_CLOSE;
            }
            if create_options.contains(FileCreateOptions::DirectoryFile) {
                nt_create_options |= FILE_DIRECTORY_FILE;
            }
            if create_options.contains(FileCreateOptions::NonDirectoryFile) {
                nt_create_options |= FILE_NON_DIRECTORY_FILE;
            }
            nt_create_options
        };
        let mut io_status_block = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
        let mut h = Default::default();
        let status = unsafe {
            NtCreateFile(
                &mut h,
                desired_access,
                &object_attributes,
                io_status_block.as_mut_ptr(),
                None,
                file_attributes,
                share_access,
                create_disposition,
                create_options,
                None,
                0,
            )
        };
        // log::debug!("NtCreateFile status: {status:?}");
        if let Err(e) = status {
            return Err(match NTSTATUS(e.code().0 & !0x1000_0000) {
                STATUS_OBJECT_NAME_NOT_FOUND => FileSystemError::ObjectNameNotFound,
                STATUS_OBJECT_NAME_COLLISION => FileSystemError::ObjectNameCollision,
                STATUS_OBJECT_NAME_INVALID => FileSystemError::ObjectNameInvalid,
                STATUS_OBJECT_PATH_NOT_FOUND => FileSystemError::ObjectPathNotFound,
                STATUS_FILE_IS_A_DIRECTORY => FileSystemError::FileIsADirectory,
                STATUS_NOT_A_DIRECTORY => FileSystemError::NotADirectory,
                STATUS_ACCESS_DENIED => FileSystemError::AccessDenied,
                _ => FileSystemError::Other(e.into()),
            });
        }
        let mut io_status_block = unsafe { io_status_block.assume_init() };
        let new_file_created = io_status_block.Information as u32 == FILE_CREATED;
        let mut file_information = MaybeUninit::<FILE_ATTRIBUTE_TAG_INFORMATION>::uninit();
        // Then check if the object is a directory
        let status = unsafe {
            // TODO: FileAttributeTagInformation seems to be missing in windows-rs?
            NtQueryInformationFile(
                h,
                &mut io_status_block,
                file_information.as_mut_ptr() as _,
                std::mem::size_of_val(&file_information) as _,
                FileAttributeTagInformation,
            )
        };
        status.map_err(|e| FileSystemError::Other(e.into()))?;
        let file_information = unsafe { file_information.assume_init() };
        let is_dir = (file_information.FileAttributes & FILE_ATTRIBUTE_DIRECTORY.0) != 0;

        Ok(super::CreateFileInfo {
            context: Box::new(LocalFsFile { h }),
            is_dir,
            new_file_created,
        })
    }
    fn get_fs_free_space(&self) -> super::FileSystemResult<super::FileSystemSpaceInfo> {
        Ok(super::FileSystemSpaceInfo {
            bytes_count: 1024 * 1024 * 1024 * 1024,
            free_bytes_count: 1024 * 1024 * 1024 * 1024 * 7 / 8,
            available_bytes_count: 1024 * 1024 * 1024 * 1024 * 7 / 8,
        })
    }
    fn get_fs_characteristics(&self) -> super::FileSystemResult<super::FileSystemCharacteristics> {
        Ok(super::FileSystemCharacteristics::empty())
    }
}

struct LocalFsFile {
    h: HANDLE,
}
unsafe impl Send for LocalFsFile {}
unsafe impl Sync for LocalFsFile {}

impl super::File for LocalFsFile {
    fn read_at(&self, offset: u64, buffer: &mut [u8]) -> super::FileSystemResult<u64> {
        let buf = buffer.as_mut_ptr();
        let len = buffer.len();
        // log::debug!("read_at: ({offset}, {len})");
        // TODO: Implement very long read support
        let len: u32 = len
            .try_into()
            .map_err(|_| FileSystemError::NotImplemented)?;
        let mut io_status_block = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
        let event = unsafe { CreateEventW(None, false, false, None) }
            .map_err(|e| FileSystemError::Other(e.into()))?;
        scopeguard::defer! {
            let _ = unsafe { CloseHandle(event) };
        };
        let status = unsafe {
            NtReadFile(
                self.h,
                event,
                None,
                None,
                io_status_block.as_mut_ptr(),
                buf as _,
                len,
                Some(&(offset as _)),
                None,
            )
        };
        // log::debug!("NtReadFile status: {status:?}");
        status.map_err(|e| FileSystemError::Other(e.into()))?;
        unsafe {
            if WaitForSingleObject(event, INFINITE) != WAIT_OBJECT_0 {
                return Err(FileSystemError::Other(anyhow::anyhow!(
                    "failed to wait for read"
                )));
            }
        }
        let io_status_block = unsafe { io_status_block.assume_init() };
        let status = unsafe { io_status_block.Anonymous.Status };
        // log::debug!("NtReadFile status: {status:?}");
        match status {
            STATUS_END_OF_FILE => (),
            _ => status.ok().map_err(|e| FileSystemError::Other(e.into()))?,
        }
        Ok(io_status_block.Information as _)
    }
    fn write_at(
        &self,
        offset: Option<u64>,
        buffer: &[u8],
        constrain_size: bool,
    ) -> super::FileSystemResult<u64> {
        let buf = buffer.as_ptr();
        let len = buffer.len();
        // TODO: Implement very long write support
        let len: u32 = len
            .try_into()
            .map_err(|_| FileSystemError::NotImplemented)?;
        let mut io_status_block = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
        let event = unsafe { CreateEventW(None, false, false, None) }
            .map_err(|e| FileSystemError::Other(e.into()))?;
        scopeguard::defer! {
            let _ = unsafe { CloseHandle(event) };
        };
        let offset = offset.unwrap_or(0xffffffff00000000 | FILE_WRITE_TO_END_OF_FILE as u64);
        if constrain_size {
            log::error!("localfs: constrain_size NOT SUPPORTED; assuming normal writings");
        }
        let status = unsafe {
            NtWriteFile(
                self.h,
                event,
                None,
                None,
                io_status_block.as_mut_ptr(),
                buf as _,
                len,
                Some(&(offset as _)),
                None,
            )
        };
        status.map_err(|e| FileSystemError::Other(e.into()))?;
        unsafe {
            if WaitForSingleObject(event, INFINITE) != WAIT_OBJECT_0 {
                return Err(FileSystemError::Other(anyhow::anyhow!(
                    "failed to wait for write"
                )));
            }
        }
        let io_status_block = unsafe { io_status_block.assume_init() };
        unsafe {
            io_status_block
                .Anonymous
                .Status
                .ok()
                .map_err(|e| FileSystemError::Other(e.into()))?;
        }
        Ok(io_status_block.Information as _)
    }
    fn flush_buffers(&self) -> super::FileSystemResult<()> {
        let mut io_status_block = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
        let status = unsafe {
            NtFlushBuffersFileEx(self.h, 0, std::ptr::null(), 0, io_status_block.as_mut_ptr())
        };
        status.map_err(|e| FileSystemError::Other(e.into()))?;
        let io_status_block = unsafe { io_status_block.assume_init() };
        Ok(())
    }
    fn get_stat(&self) -> super::FileSystemResult<super::FileStatInfo> {
        // NOTE: We issue multiple syscalls, just like ReactOS does
        let mut io_status_block = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
        let mut basic_info = MaybeUninit::<FILE_BASIC_INFORMATION>::uninit();
        unsafe {
            NtQueryInformationFile(
                self.h,
                io_status_block.as_mut_ptr(),
                basic_info.as_mut_ptr() as _,
                std::mem::size_of_val(&basic_info) as _,
                FileBasicInformation,
            )
            .map_err(|e| FileSystemError::Other(e.into()))?;
        }
        let basic_info = unsafe { basic_info.assume_init() };
        let mut standard_info = MaybeUninit::<FILE_STANDARD_INFORMATION>::uninit();
        unsafe {
            NtQueryInformationFile(
                self.h,
                io_status_block.as_mut_ptr(),
                standard_info.as_mut_ptr() as _,
                std::mem::size_of_val(&standard_info) as _,
                FileStandardInformation,
            )
            .map_err(|e| FileSystemError::Other(e.into()))?;
        }
        let standard_info = unsafe { standard_info.assume_init() };
        let mut internal_info = MaybeUninit::<FILE_INTERNAL_INFORMATION>::uninit();
        unsafe {
            NtQueryInformationFile(
                self.h,
                io_status_block.as_mut_ptr(),
                internal_info.as_mut_ptr() as _,
                std::mem::size_of_val(&internal_info) as _,
                FileInternalInformation,
            )
            .map_err(|e| FileSystemError::Other(e.into()))?;
        }
        let internal_info = unsafe { internal_info.assume_init() };
        let file_attributes = nt_file_attributes_to_local(basic_info.FileAttributes);
        Ok(super::FileStatInfo {
            index: internal_info.IndexNumber as _,
            size: standard_info.EndOfFile as _,
            is_dir: file_attributes.contains(FileAttributes::DirectoryFile),
            attributes: file_attributes,
            creation_time: unsafe { std::mem::transmute(basic_info.CreationTime) },
            last_access_time: unsafe { std::mem::transmute(basic_info.LastAccessTime) },
            last_write_time: unsafe { std::mem::transmute(basic_info.LastWriteTime) },
        })
    }
    fn set_end_of_file(&self, offset: u64) -> super::FileSystemResult<()> {
        let file_info = FILE_END_OF_FILE_INFORMATION {
            EndOfFile: offset as _,
        };
        let mut io_status_block = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
        let status = unsafe {
            NtSetInformationFile(
                self.h,
                io_status_block.as_mut_ptr(),
                &file_info as *const _ as _,
                std::mem::size_of_val(&file_info) as _,
                FileEndOfFileInformation,
            )
        };
        status.map_err(|e| FileSystemError::Other(e.into()))?;
        let io_status_block = unsafe { io_status_block.assume_init() };
        Ok(())
    }
    fn set_file_times(
        &self,
        creation_time: std::time::SystemTime,
        last_access_time: std::time::SystemTime,
        last_write_time: std::time::SystemTime,
    ) -> super::FileSystemResult<()> {
        let file_info = FILE_BASIC_INFORMATION {
            CreationTime: unsafe { std::mem::transmute(creation_time) },
            LastAccessTime: unsafe { std::mem::transmute(last_access_time) },
            LastWriteTime: unsafe { std::mem::transmute(last_write_time) },
            ChangeTime: 0,
            FileAttributes: 0,
        };
        let mut io_status_block = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
        let status = unsafe {
            NtSetInformationFile(
                self.h,
                io_status_block.as_mut_ptr(),
                &file_info as *const _ as _,
                std::mem::size_of_val(&file_info) as _,
                FileBasicInformation,
            )
        };
        status.map_err(|e| FileSystemError::Other(e.into()))?;
        let io_status_block = unsafe { io_status_block.assume_init() };
        Ok(())
    }
    fn set_delete(&self, delete_on_close: bool) -> super::FileSystemResult<()> {
        let file_info = FILE_DISPOSITION_INFORMATION {
            DeleteFile: delete_on_close.into(),
        };
        let mut io_status_block = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
        let status = unsafe {
            NtSetInformationFile(
                self.h,
                io_status_block.as_mut_ptr(),
                &file_info as *const _ as _,
                std::mem::size_of_val(&file_info) as _,
                FileDispositionInformation,
            )
        };
        status.map_err(|e| FileSystemError::Other(e.into()))?;
        let io_status_block = unsafe { io_status_block.assume_init() };
        Ok(())
    }
    fn move_to(
        &self,
        new_path: super::SegPath,
        replace_if_exists: bool,
    ) -> super::FileSystemResult<()> {
        // TODO: move_to
        // TODO: Use FileRenameInformation
        // TODO: Should we handle movement across different volumes?
        Err(FileSystemError::NotImplemented)
    }
    fn find_files_with_pattern(
        &self,
        pattern: &dyn super::FilePattern,
        filler: &mut dyn super::FindFilesDataFiller,
    ) -> super::FileSystemResult<()> {
        // TODO: Let super leak pattern string details (get_pattern_str)
        //       to speed up file listing

        // 4KB buffer for file information, should be large enough
        let mut buf = MaybeUninit::<[u8; 1024 * 4]>::uninit();

        let mut io_status_block = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
        let event = unsafe { CreateEventW(None, false, false, None) }
            .map_err(|e| FileSystemError::Other(e.into()))?;
        scopeguard::defer! {
            let _ = unsafe { CloseHandle(event) };
        };
        let filename = unsafe {
            let filename = PCWSTR::from_raw(widestring::u16cstr!("*").as_ptr());
            let mut us = MaybeUninit::<UNICODE_STRING>::uninit();
            RtlInitUnicodeStringEx(us.as_mut_ptr(), filename)
                .map_err(|e| FileSystemError::Other(e.into()))?;
            us.assume_init()
        };
        let status = unsafe {
            NtQueryDirectoryFile(
                self.h,
                event,
                None,
                None,
                io_status_block.as_mut_ptr(),
                buf.as_mut_ptr() as _,
                std::mem::size_of_val(&buf) as _,
                FileDirectoryInformation,
                BOOLEAN::from(false),
                Some(&filename),
                BOOLEAN::from(true),
            )
        };
        status.map_err(|e| FileSystemError::Other(e.into()))?;
        unsafe {
            if WaitForSingleObject(event, INFINITE) != WAIT_OBJECT_0 {
                return Err(FileSystemError::Other(anyhow::anyhow!(
                    "failed to wait for directory list"
                )));
            }
        }
        let mut io_status_block = unsafe { io_status_block.assume_init() };
        unsafe {
            io_status_block
                .Anonymous
                .Status
                .ok()
                .map_err(|e| FileSystemError::Other(e.into()))?;
        }
        loop {
            // Fill results
            let mut entry_ptr = buf.as_ptr() as *const FILE_DIRECTORY_INFORMATION;
            loop {
                let entry = unsafe { &*entry_ptr };
                // Fill
                let name = unsafe {
                    widestring::U16Str::from_ptr(
                        entry.FileName.as_ptr(),
                        (entry.FileNameLength / 2) as _,
                    )
                };
                let name = name.to_string_lossy();
                if pattern.check_name(&name) {
                    // TODO: Use correct index
                    let file_attr = nt_file_attributes_to_local(entry.FileAttributes);
                    let stat = super::FileStatInfo {
                        index: entry.FileIndex as _,
                        size: entry.EndOfFile as _,
                        is_dir: file_attr.contains(FileAttributes::DirectoryFile),
                        attributes: file_attr,
                        creation_time: unsafe { std::mem::transmute(entry.CreationTime) },
                        last_access_time: unsafe { std::mem::transmute(entry.LastAccessTime) },
                        last_write_time: unsafe { std::mem::transmute(entry.LastWriteTime) },
                    };

                    if filler.fill_data(&name, &stat).is_err() {
                        log::warn!("Failed to fill object data");
                    }
                }

                // Go to next entry
                match entry.NextEntryOffset {
                    0 => break,
                    offset @ _ => unsafe {
                        entry_ptr = (entry_ptr as *const u8).add(offset as _) as _;
                    },
                }
            }

            // List more files
            let status = unsafe {
                NtQueryDirectoryFile(
                    self.h,
                    event,
                    None,
                    None,
                    &mut io_status_block,
                    buf.as_mut_ptr() as _,
                    std::mem::size_of_val(&buf) as _,
                    FileDirectoryInformation,
                    BOOLEAN::from(false),
                    None,
                    BOOLEAN::from(false),
                )
            };
            status.map_err(|e| FileSystemError::Other(e.into()))?;
            unsafe {
                if WaitForSingleObject(event, INFINITE) != WAIT_OBJECT_0 {
                    return Err(FileSystemError::Other(anyhow::anyhow!(
                        "failed to wait for directory list"
                    )));
                }
            }
            let status = unsafe { io_status_block.Anonymous.Status };
            match status {
                STATUS_NO_MORE_FILES => break,
                _ => status.ok().map_err(|e| FileSystemError::Other(e.into()))?,
            }
            if io_status_block.Information == 0 {
                return Err(FileSystemError::Other(anyhow::anyhow!(
                    "directory list buffer too small"
                )));
            }
        }
        Ok(())
    }
}

impl Drop for LocalFsFile {
    fn drop(&mut self) {
        unsafe {
            let _ = NtClose(self.h);
        }
    }
}

impl LocalFsHandler {
    fn new() -> Self {
        LocalFsHandler {}
    }
}

pub struct LocalFsProvider {}
impl super::FsProvider for LocalFsProvider {
    fn get_id(&self) -> Uuid {
        LOCALFS_ID
    }
    fn get_name(&self) -> &'static str {
        "local"
    }
    fn get_version(&self) -> (u32, u32, u32) {
        (0, 1, 0)
    }
    fn construct(
        &self,
        config: serde_json::Value,
        ctx: &mut dyn super::FileSystemCreationContext,
    ) -> Result<std::sync::Arc<dyn super::FileSystemHandler>, super::FileSystemCreationError> {
        Ok(Arc::new(LocalFsHandler::new()))
    }
    fn get_template_config(&self) -> serde_json::Value {
        serde_json::Value::Null
    }
}

impl LocalFsProvider {
    pub fn new() -> Self {
        LocalFsProvider {}
    }
}
