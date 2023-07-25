mod operations;

use std::{
    marker::PhantomPinned,
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc,
    },
};

use uuid::{uuid, Uuid};
use widestring::U16CStr;

use dokan_sys::*;

use atomic_wait::*;

use super::FileSystemServer;
use crate::fs_provider::FileSystemHandler;

pub const DOKAN_FSERVER_ID: Uuid = uuid!("40612005-FA2F-49B8-820B-B0E7521602D7");

#[derive(thiserror::Error, Debug)]
#[repr(i32)]
enum DokanError {
    #[error("general error")]
    General = -1,
    #[error("bad drive letter")]
    DriveLetter = -2,
    #[error("can't install driver")]
    DriverInstall = -3,
    #[error("the driver responds that something is wrong")]
    Start = -4,
    #[error("can't assign a drive letter or mount point, probably already used by another volume")]
    Mount = -5,
    #[error("the mount point is invalid")]
    MountPoint = -6,
    #[error("requested an incompatible version")]
    Version = -7,
}

impl From<i32> for DokanError {
    fn from(value: i32) -> Self {
        match value {
            DOKAN_ERROR => Self::General,
            DOKAN_DRIVE_LETTER_ERROR => Self::DriveLetter,
            DOKAN_DRIVER_INSTALL_ERROR => Self::DriverInstall,
            DOKAN_START_ERROR => Self::Start,
            DOKAN_MOUNT_ERROR => Self::Mount,
            DOKAN_MOUNT_POINT_ERROR => Self::MountPoint,
            DOKAN_VERSION_ERROR => Self::Version,
            _ => Self::General,
        }
    }
}

struct DokanFServer {
    handle: DOKAN_HANDLE,
    shutdown_flag: AtomicU32,
    fs: Arc<dyn FileSystemHandler>,
    // TODO: Is data movable inside Arc?
    pin: PhantomPinned,
}

impl DokanFServer {
    fn new(mount_point: &U16CStr, fs: Arc<dyn FileSystemHandler>) -> Result<Arc<Self>, DokanError> {
        let mut server = Arc::new(DokanFServer {
            handle: std::ptr::null_mut(),
            shutdown_flag: AtomicU32::new(0),
            fs,
            pin: PhantomPinned,
        });
        // TODO: Remove this
        // crate::util::real_wait(&server.shutdown_flag, 0);
        let mut handle: DOKAN_HANDLE = std::ptr::null_mut();
        let mut options = DOKAN_OPTIONS {
            Version: DOKAN_VERSION as _,
            SingleThread: false.into(),
            Options: DOKAN_OPTION_MOUNT_MANAGER,
            GlobalContext: Arc::as_ptr(&server) as _,
            MountPoint: mount_point.as_ptr(),
            UNCName: std::ptr::null(),
            Timeout: 0,
            AllocationUnitSize: 0,
            SectorSize: 0,
            VolumeSecurityDescriptorLength: 0,
            VolumeSecurityDescriptor: unsafe { std::mem::zeroed() },
        };
        let mut operations = DOKAN_OPERATIONS {
            ZwCreateFile: Some(operations::create_file),
            Cleanup: Some(operations::cleanup),
            CloseFile: Some(operations::close_file),
            ReadFile: Some(operations::read_file),
            WriteFile: Some(operations::write_file),
            FlushFileBuffers: Some(operations::flush_file_buffers),
            GetFileInformation: Some(operations::get_file_information),
            FindFiles: None,
            FindFilesWithPattern: Some(operations::find_files_with_pattern),
            SetFileAttributes: Some(operations::set_file_attributes),
            SetFileTime: Some(operations::set_file_time),
            DeleteFile: Some(operations::delete_file),
            DeleteDirectory: Some(operations::delete_directory),
            MoveFile: Some(operations::move_file),
            SetEndOfFile: Some(operations::set_end_of_file),
            SetAllocationSize: Some(operations::set_allocation_size),
            LockFile: None,
            UnlockFile: None,
            GetDiskFreeSpace: Some(operations::get_disk_free_space),
            // GetVolumeInformation: Some(operations::get_volume_information),
            GetVolumeInformation: None,
            Mounted: Some(operations::mounted),
            Unmounted: Some(operations::unmounted),
            GetFileSecurity: None,
            SetFileSecurity: None,
            FindStreams: None,
        };
        let result = unsafe { DokanCreateFileSystem(&mut options, &mut operations, &mut handle) };
        if result != DOKAN_SUCCESS {
            return Err(DokanError::from(result));
        }
        Arc::get_mut(&mut server).unwrap().handle = handle;
        Ok(server)
    }
}

impl FileSystemServer for DokanFServer {}

impl Drop for DokanFServer {
    fn drop(&mut self) {
        unsafe {
            DokanCloseHandle(self.handle);
        }
        crate::util::real_wait(&self.shutdown_flag, 0);
    }
}

pub struct DokanFServerProvider {}
impl super::FsServerProvider for DokanFServerProvider {
    fn get_id(&self) -> Uuid {
        DOKAN_FSERVER_ID
    }
    fn get_name(&self) -> &'static str {
        "Dokan Disk Mounter"
    }
    fn construct(
        &self,
        fs: Arc<dyn crate::fs_provider::FileSystemHandler>,
    ) -> anyhow::Result<Arc<dyn FileSystemServer>> {
        let result = DokanFServer::new(widestring::u16cstr!("M:"), fs)?;
        Ok(result)
    }
}

impl DokanFServerProvider {
    pub fn new() -> Self {
        Self {}
    }
}
