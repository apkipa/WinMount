mod operations;

use std::{
    marker::PhantomPinned,
    mem::MaybeUninit,
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc,
    },
};

use uuid::{uuid, Uuid};

use dokan_sys::*;

// use winapi::um::winbase::INFINITE;

use super::FileSystemServer;
use crate::fs_provider::{FileSystemCharacteristics, FileSystemHandler};

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

static mut OPERATIONS: DOKAN_OPERATIONS = DOKAN_OPERATIONS {
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

// TODO: Consider pinning the struct due to Dokan requirements
struct DokanFServer {
    handle: DOKAN_HANDLE,
    shutdown_flag: AtomicU32,
    fs: Arc<dyn FileSystemHandler>,
    dokan_options: MaybeUninit<DOKAN_OPTIONS>,
    open_objs: scc::HashSet<u64>,
    block_sys_dirs: bool,
    pin: PhantomPinned,
}

impl DokanFServer {
    fn new(
        fs: Arc<dyn FileSystemHandler>,
        config: DokanFServerConfig,
    ) -> Result<Arc<Self>, DokanError> {
        let mount_point = widestring::U16CString::from_str_truncate(config.mount_point);

        let mut server = Arc::new(DokanFServer {
            handle: std::ptr::null_mut(),
            shutdown_flag: AtomicU32::new(0),
            fs,
            dokan_options: MaybeUninit::uninit(),
            open_objs: scc::HashSet::new(),
            block_sys_dirs: !config.enable_sys_dirs,
            pin: PhantomPinned,
        });
        let mut handle: DOKAN_HANDLE = std::ptr::null_mut();
        let global_context = Arc::as_ptr(&server) as _;
        let options = Arc::get_mut(&mut server)
            .unwrap()
            .dokan_options
            .write(DOKAN_OPTIONS {
                Version: DOKAN_VERSION as _,
                SingleThread: false.into(),
                Options: {
                    let mut options = DOKAN_OPTION_MOUNT_MANAGER;
                    if config.readonly_drive {
                        options |= DOKAN_OPTION_WRITE_PROTECT;
                    }
                    if config.enable_debug {
                        options |= DOKAN_OPTION_DEBUG
                            | DOKAN_OPTION_STDERR
                            | DOKAN_OPTION_DISPATCH_DRIVER_LOGS;
                    }
                    options
                },
                GlobalContext: global_context,
                MountPoint: mount_point.as_ptr(),
                UNCName: std::ptr::null(),
                Timeout: 0,
                AllocationUnitSize: 0,
                SectorSize: 0,
                VolumeSecurityDescriptorLength: 0,
                VolumeSecurityDescriptor: unsafe { std::mem::zeroed() },
            });
        // SAFETY: Dokan doesn't actully mutate referenced variables, nor do we
        let result = unsafe { DokanCreateFileSystem(options, &mut OPERATIONS, &mut handle) };
        if result != DOKAN_SUCCESS {
            return Err(DokanError::from(result));
        }
        Arc::get_mut(&mut server).unwrap().handle = handle;
        Ok(server)
    }

    fn owned_file_to_u64(file: crate::fs_provider::OwnedFile) -> u64 {
        Box::into_raw(Box::new(file)) as _
    }
    unsafe fn u64_to_owned_file<'f>(file: u64) -> Box<crate::fs_provider::OwnedFile<'f>> {
        Box::<crate::fs_provider::OwnedFile<'f>>::from_raw(file as _)
    }
    fn add_open_obj_ptr(&self, obj: u64) {
        self.open_objs
            .insert(obj)
            .expect("duplicate open objects must not exist");
    }
    // NOTE: Removes entries WITHOUT dropping
    fn remove_open_obj_ptr(&self, obj: u64) {
        self.open_objs
            .remove(&obj)
            .expect("object must exist in open objects list");
    }
    fn drop_open_objs(&mut self) {
        // SAFETY: Files are uniquely returned since we have exclusive access
        self.open_objs.retain(|&k| {
            let _ = unsafe { Self::u64_to_owned_file(k) };
            false
        });
    }
}

unsafe impl Send for DokanFServer {}
unsafe impl Sync for DokanFServer {}

impl FileSystemServer for DokanFServer {}

impl Drop for DokanFServer {
    fn drop(&mut self) {
        unsafe {
            DokanCloseHandle(self.handle);
        }
        crate::util::real_wait(&self.shutdown_flag, 0);
        // unsafe {
        //     DokanRemoveMountPoint(widestring::u16cstr!("M:\\").as_ptr());
        //     DokanWaitForFileSystemClosed(self.handle, INFINITE);
        //     DokanCloseHandle(self.handle);
        // }

        if !self.open_objs.is_empty() {
            log::warn!(
                "Dokan did not properly cleanup resources ({} leaked). Cleaning up manually...",
                self.open_objs.len()
            );
            self.drop_open_objs();
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct DokanFServerConfig {
    /// Where to mount the filesystem at (such as C:\\, etc.).
    mount_point: String,
    /// Whether to allow system directories to be created and accessed. It's strongly
    /// discouraged to enable this, as it can possibly mess up the filesystem.
    enable_sys_dirs: bool,
    /// Mount filesystem as read-only.
    readonly_drive: bool,
    /// Enable Dokan debug output.
    enable_debug: bool,
}

pub struct DokanFServerProvider {}
impl super::FsServerProvider for DokanFServerProvider {
    fn get_id(&self) -> Uuid {
        DOKAN_FSERVER_ID
    }
    fn get_name(&self) -> &'static str {
        "Dokan Disk Mounter"
    }
    fn get_version(&self) -> (u32, u32, u32) {
        (0, 1, 0)
    }
    fn construct(
        &self,
        fs: Arc<dyn crate::fs_provider::FileSystemHandler>,
        config: serde_json::Value,
    ) -> anyhow::Result<Arc<dyn FileSystemServer>> {
        // TODO: Reject when requested mount point has been taken
        let mut config: DokanFServerConfig = serde_json::from_value(config)?;
        // Override some config
        let fs_chars = fs.get_fs_characteristics()?;
        if fs_chars.contains(FileSystemCharacteristics::ReadOnly) {
            config.readonly_drive = true;
        }
        let result = DokanFServer::new(fs, config)?;
        Ok(result)
    }
    fn get_template_config(&self) -> serde_json::Value {
        // NOTE: We should not allow user to use a spare mount point when
        //       the specified one has been taken
        serde_json::to_value(DokanFServerConfig {
            mount_point: "M:\\".to_owned(),
            enable_sys_dirs: false,
            readonly_drive: false,
            enable_debug: false,
        })
        .unwrap()
    }
}

impl DokanFServerProvider {
    pub fn new() -> Self {
        Self {}
    }
}
