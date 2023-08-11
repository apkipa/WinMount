use std::panic::UnwindSafe;
use std::sync::atomic::Ordering;

use dokan_sys::PDOKAN_IO_SECURITY_CONTEXT;
use dokan_sys::{win32::*, *};
use widestring::U16CStr;
use winapi::shared::minwindef::{BOOL, DWORD, FILETIME, LPCVOID, LPDWORD, LPVOID, MAX_PATH};
use winapi::shared::ntdef::{LONGLONG, LPWSTR, PULONGLONG};
use winapi::um::fileapi::{BY_HANDLE_FILE_INFORMATION, LPBY_HANDLE_FILE_INFORMATION};
use winapi::um::minwinbase::WIN32_FIND_DATAW;
use winapi::um::winnt::{
    FILE_APPEND_DATA, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_NORMAL,
    FILE_ATTRIBUTE_READONLY, FILE_EXECUTE, FILE_READ_DATA, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, FILE_WRITE_DATA,
};
use winapi::{
    shared::{
        minwindef::ULONG,
        ntdef::{LPCWSTR, NTSTATUS},
        ntstatus::*,
    },
    um::winnt::ACCESS_MASK,
};

use crate::fs_provider::FileSystemError;
use crate::fs_provider::{
    FileAttributes, FileCreateDisposition, FileCreateOptions, FileDesiredAccess, FileShareAccess,
    OwnedFile, PathDelimiter, U16SegPath,
};

const DOKAN_VOLUME_ID: DWORD = 0x19831116;

fn fs_error_to_ntstatus(err: FileSystemError) -> NTSTATUS {
    match err {
        FileSystemError::ObjectPathNotFound => STATUS_OBJECT_PATH_NOT_FOUND,
        FileSystemError::NotImplemented => STATUS_NOT_IMPLEMENTED,
        FileSystemError::FileIsADirectory => STATUS_FILE_IS_A_DIRECTORY,
        FileSystemError::NotADirectory => STATUS_NOT_A_DIRECTORY,
        FileSystemError::ObjectNameNotFound => STATUS_OBJECT_NAME_NOT_FOUND,
        FileSystemError::ObjectNameCollision => STATUS_OBJECT_NAME_COLLISION,
        FileSystemError::DirectoryNotEmpty => STATUS_DIRECTORY_NOT_EMPTY,
        FileSystemError::AccessDenied => STATUS_ACCESS_DENIED,
        FileSystemError::NoSuchFile => STATUS_NO_SUCH_FILE,
        _ => STATUS_INTERNAL_ERROR,
    }
}

fn wrap_ffi(func: impl FnOnce() -> Result<(), FileSystemError> + UnwindSafe) -> NTSTATUS {
    std::panic::catch_unwind(func)
        .map(|x| match x {
            Ok(_) => STATUS_SUCCESS,
            Err(e) => fs_error_to_ntstatus(e),
        })
        .unwrap_or(STATUS_INTERNAL_ERROR)
}

fn wrap_unit_ffi(func: impl FnOnce() + UnwindSafe) {
    let _ = std::panic::catch_unwind(func);
}

fn server_from_dokan_file_info<'s>(dokan_file_info: &DOKAN_FILE_INFO) -> &'s super::DokanFServer {
    unsafe { &*((*dokan_file_info.DokanOptions).GlobalContext as *const _) }
}

// TODO: Figure out the proper lifetime
unsafe fn file_from_dokan_file_info<'f>(
    dokan_file_info: &DOKAN_FILE_INFO,
) -> &'static OwnedFile<'f> {
    assert!(dokan_file_info.Context != 0);
    unsafe { &*(dokan_file_info.Context as *const _) }
}

fn set_file_into_dokan_file_info(file: OwnedFile, dokan_file_info: &mut DOKAN_FILE_INFO) {
    drop_file_from_dokan_file_info(dokan_file_info);
    let file = super::DokanFServer::owned_file_to_u64(file);
    server_from_dokan_file_info(dokan_file_info).add_open_obj_ptr(file);
    dokan_file_info.Context = file;
}
fn drop_file_from_dokan_file_info(dokan_file_info: &mut DOKAN_FILE_INFO) {
    if dokan_file_info.Context == 0 {
        return;
    }
    let file = dokan_file_info.Context;
    server_from_dokan_file_info(dokan_file_info).remove_open_obj_ptr(file);
    let _ = unsafe { super::DokanFServer::u64_to_owned_file(file) };
    dokan_file_info.Context = 0;
}

pub(super) extern "stdcall" fn create_file(
    file_name: LPCWSTR,
    security_context: PDOKAN_IO_SECURITY_CONTEXT,
    desired_access: ACCESS_MASK,
    file_attributes: ULONG,
    share_access: ULONG,
    create_disposition: ULONG,
    create_options: ULONG,
    dokan_file_info: PDOKAN_FILE_INFO,
) -> NTSTATUS {
    use widestring::u16cstr;
    wrap_ffi(|| {
        let dokan_file_info = unsafe { &mut *dokan_file_info };
        let server = server_from_dokan_file_info(dokan_file_info);
        let file_name = unsafe { U16CStr::from_ptr_str(file_name) };

        if log::log_enabled!(log::Level::Trace) {
            log::trace!("Opening object `{}`", file_name.to_string_lossy());
        }

        // Block access to system folders
        if file_name == u16cstr!("\\$RECYCLE.BIN")
            || file_name == u16cstr!("\\System Volume Information")
        {
            return Err(FileSystemError::NoSuchFile);
        }
        let file_name = U16SegPath::new(file_name, PathDelimiter::BackSlash);
        let raw_create_disposition = create_disposition;
        let create_disposition = match create_disposition {
            FILE_CREATE => FileCreateDisposition::CreateNew,
            FILE_OPEN => FileCreateDisposition::OpenExisting,
            FILE_OPEN_IF => FileCreateDisposition::OpenAlways,
            FILE_OVERWRITE => FileCreateDisposition::TruncateExisting,
            FILE_SUPERSEDE | FILE_OVERWRITE_IF => FileCreateDisposition::CreateAlways,
            _ => return Err(FileSystemError::InvalidParameter),
        };
        let create_options = {
            let mut x = FileCreateOptions::empty();
            // TODO: Should dokan_file_info.IsDirectory be honored?
            if (create_options & FILE_DIRECTORY_FILE) != 0 {
                x |= FileCreateOptions::DirectoryFile;
            }
            // TODO: Should dokan_file_info.DeleteOnClose be honored?
            if (create_options & FILE_DELETE_ON_CLOSE) != 0 {
                x |= FileCreateOptions::DeleteOnClose;
            }
            x
        };
        let share_access = {
            let mut x = FileShareAccess::empty();
            if (share_access & FILE_SHARE_READ) != 0 {
                x |= FileShareAccess::Read;
            }
            if (share_access & FILE_SHARE_WRITE) != 0 {
                x |= FileShareAccess::Write;
            }
            if (share_access & FILE_SHARE_DELETE) != 0 {
                x |= FileShareAccess::Delete;
            }
            x
        };
        let file_attributes = {
            let mut x = FileAttributes::empty();
            if (file_attributes & FILE_ATTRIBUTE_HIDDEN) != 0 {
                x |= FileAttributes::Hidden;
            }
            if (file_attributes & FILE_ATTRIBUTE_READONLY) != 0 {
                x |= FileAttributes::Readonly;
            }
            x
        };
        let desired_access = {
            let mut x = FileDesiredAccess::empty();
            if (desired_access & (FILE_WRITE_DATA | FILE_APPEND_DATA)) != 0 {
                x |= FileDesiredAccess::Write;
            }
            if (desired_access & FILE_READ_DATA) != 0 {
                x |= FileDesiredAccess::Read;
            }
            if (desired_access & FILE_EXECUTE) != 0 {
                x |= FileDesiredAccess::Execute;
            }
            x
        };
        let result = server.fs.wide_create_file(
            file_name,
            desired_access,
            file_attributes,
            share_access,
            create_disposition,
            create_options,
        )?;
        dokan_file_info.IsDirectory = result.is_dir.into();
        set_file_into_dokan_file_info(result.context, dokan_file_info);
        if (raw_create_disposition == FILE_CREATE
            || raw_create_disposition == FILE_OPEN_IF
            || raw_create_disposition == FILE_OVERWRITE_IF
            || raw_create_disposition == FILE_SUPERSEDE)
            && !result.new_file_created
        {
            Err(FileSystemError::ObjectNameCollision)
        } else {
            Ok(())
        }
    })
}

pub(super) extern "stdcall" fn cleanup(file_name: LPCWSTR, dokan_file_info: PDOKAN_FILE_INFO) {
    wrap_unit_ffi(|| {
        if log::log_enabled!(log::Level::Trace) {
            let file_name = unsafe { U16CStr::from_ptr_str(file_name) };
            log::trace!("Cleaning up object `{}`", file_name.to_string_lossy());
        }

        let dokan_file_info = unsafe { &mut *dokan_file_info };
        if dokan_file_info.DeleteOnClose != 0 {
            drop_file_from_dokan_file_info(dokan_file_info);
        }
    })
}

pub(super) extern "stdcall" fn close_file(file_name: LPCWSTR, dokan_file_info: PDOKAN_FILE_INFO) {
    wrap_unit_ffi(|| {
        if log::log_enabled!(log::Level::Trace) {
            let file_name = unsafe { U16CStr::from_ptr_str(file_name) };
            log::trace!("Closing object `{}`", file_name.to_string_lossy());
        }

        let dokan_file_info = unsafe { &mut *dokan_file_info };
        drop_file_from_dokan_file_info(dokan_file_info);
    })
}

pub(super) extern "stdcall" fn read_file(
    file_name: LPCWSTR,
    buffer: LPVOID,
    buffer_length: DWORD,
    read_length: LPDWORD,
    offset: LONGLONG,
    dokan_file_info: PDOKAN_FILE_INFO,
) -> NTSTATUS {
    wrap_ffi(|| unsafe {
        let dokan_file_info = &mut *dokan_file_info;
        let file = file_from_dokan_file_info(dokan_file_info);
        let mut buf: std::io::IoSliceMut = std::mem::transmute(winapi::shared::ws2def::WSABUF {
            len: buffer_length,
            buf: buffer as _,
        });
        let final_len = file.read_at(offset as _, buf.as_mut())?;
        read_length.write(final_len as _);
        Ok(())
    })
}

pub(super) extern "stdcall" fn write_file(
    file_name: LPCWSTR,
    buffer: LPCVOID,
    number_of_bytes_to_write: DWORD,
    number_of_bytes_written: LPDWORD,
    offset: LONGLONG,
    dokan_file_info: PDOKAN_FILE_INFO,
) -> NTSTATUS {
    wrap_ffi(|| unsafe {
        let dokan_file_info = &mut *dokan_file_info;
        let file = file_from_dokan_file_info(dokan_file_info);
        let buf: std::io::IoSlice = std::mem::transmute(winapi::shared::ws2def::WSABUF {
            len: number_of_bytes_to_write,
            buf: buffer as _,
        });
        let offset = match dokan_file_info.WriteToEndOfFile {
            0 => Some(offset as _),
            _ => None,
        };
        let is_constrained_write = dokan_file_info.PagingIo != 0;
        let final_len = file.write_at(offset, buf.as_ref(), is_constrained_write)?;
        number_of_bytes_written.write(final_len as _);
        Ok(())
    })
}

pub(super) extern "stdcall" fn flush_file_buffers(
    file_name: LPCWSTR,
    dokan_file_info: PDOKAN_FILE_INFO,
) -> NTSTATUS {
    wrap_ffi(|| unsafe {
        let dokan_file_info = &mut *dokan_file_info;
        let file = file_from_dokan_file_info(dokan_file_info);
        file.flush_buffers()?;
        Ok(())
    })
}

pub(super) extern "stdcall" fn get_file_information(
    file_name: LPCWSTR,
    buffer: LPBY_HANDLE_FILE_INFORMATION,
    dokan_file_info: PDOKAN_FILE_INFO,
) -> NTSTATUS {
    wrap_ffi(|| unsafe {
        let dokan_file_info = &mut *dokan_file_info;
        let file = file_from_dokan_file_info(dokan_file_info);
        let stat = file.get_stat()?;
        buffer.write(BY_HANDLE_FILE_INFORMATION {
            dwFileAttributes: if stat.is_dir {
                FILE_ATTRIBUTE_DIRECTORY
            } else {
                FILE_ATTRIBUTE_NORMAL
            },
            ftCreationTime: std::mem::transmute(stat.creation_time),
            ftLastAccessTime: std::mem::transmute(stat.last_access_time),
            ftLastWriteTime: std::mem::transmute(stat.last_write_time),
            dwVolumeSerialNumber: DOKAN_VOLUME_ID,
            nFileSizeHigh: (stat.size >> 32) as _,
            nFileSizeLow: stat.size as _,
            nNumberOfLinks: 1,
            nFileIndexHigh: (stat.index >> 32) as _,
            nFileIndexLow: stat.index as _,
        });
        Ok(())
    })
}

pub(super) extern "stdcall" fn find_files_with_pattern(
    file_name: LPCWSTR,
    search_pattern: LPCWSTR,
    fill_find_data: PFillFindData,
    dokan_file_info: PDOKAN_FILE_INFO,
) -> NTSTATUS {
    wrap_ffi(|| unsafe {
        let dokan_file_info = &mut *dokan_file_info;
        let file = file_from_dokan_file_info(dokan_file_info);
        struct DokanFilePattern {
            pattern: LPCWSTR,
            ignore_case: bool,
        }
        impl crate::fs_provider::WideFilePattern for DokanFilePattern {
            fn check_name(&self, name: &U16CStr) -> bool {
                unsafe {
                    DokanIsNameInExpression(self.pattern, name.as_ptr(), self.ignore_case.into())
                        != 0
                }
            }
        }
        let file_pattern = DokanFilePattern {
            pattern: search_pattern,
            ignore_case: true,
        };
        struct DokanFileDataFiller {
            fill_find_data: PFillFindData,
            dokan_file_info: PDOKAN_FILE_INFO,
        }
        impl crate::fs_provider::WideFindFilesDataFiller for DokanFileDataFiller {
            fn fill_data(
                &mut self,
                name: &U16CStr,
                stat: &crate::fs_provider::FileStatInfo,
            ) -> Result<(), ()> {
                unsafe {
                    let mut find_data = WIN32_FIND_DATAW {
                        dwFileAttributes: if stat.is_dir {
                            FILE_ATTRIBUTE_DIRECTORY
                        } else {
                            FILE_ATTRIBUTE_NORMAL
                        },
                        ftCreationTime: std::mem::transmute(stat.creation_time),
                        ftLastAccessTime: std::mem::transmute(stat.last_access_time),
                        ftLastWriteTime: std::mem::transmute(stat.last_write_time),
                        nFileSizeHigh: (stat.size >> 32) as _,
                        nFileSizeLow: stat.size as _,
                        dwReserved0: 0,
                        dwReserved1: 0,
                        cFileName: {
                            let mut buf = [0; MAX_PATH];
                            let copy_len = name.len();
                            if copy_len > buf.len() - 1 {
                                return Err(());
                            }
                            std::ptr::copy_nonoverlapping(
                                name.as_ptr(),
                                buf.as_mut_ptr(),
                                copy_len,
                            );
                            buf
                        },
                        cAlternateFileName: std::mem::zeroed(),
                    };
                    if (self.fill_find_data)(&mut find_data, self.dokan_file_info) == 0 {
                        Ok(())
                    } else {
                        Err(())
                    }
                }
            }
        }
        let mut data_filler = DokanFileDataFiller {
            fill_find_data,
            dokan_file_info,
        };
        file.wide_find_files_with_pattern(&file_pattern, &mut data_filler)?;
        Ok(())
    })
}

pub(super) extern "stdcall" fn set_file_attributes(
    file_name: LPCWSTR,
    file_attributes: DWORD,
    dokan_file_info: PDOKAN_FILE_INFO,
) -> NTSTATUS {
    // TODO: set_file_attributes
    wrap_ffi(|| unsafe {
        let dokan_file_info = &mut *dokan_file_info;
        let file = file_from_dokan_file_info(dokan_file_info);
        // TODO...
        log::debug!("set_file_attributes is stubbed");
        Ok(())
    })
}

pub(super) extern "stdcall" fn set_file_time(
    file_name: LPCWSTR,
    creation_time: *const FILETIME,
    last_access_time: *const FILETIME,
    last_write_time: *const FILETIME,
    dokan_file_info: PDOKAN_FILE_INFO,
) -> NTSTATUS {
    wrap_ffi(|| unsafe {
        let dokan_file_info = &mut *dokan_file_info;
        let file = file_from_dokan_file_info(dokan_file_info);
        file.set_file_times(
            std::mem::transmute(*creation_time),
            std::mem::transmute(*last_access_time),
            std::mem::transmute(*last_write_time),
        )?;
        Ok(())
    })
}

pub(super) extern "stdcall" fn delete_file(
    file_name: LPCWSTR,
    dokan_file_info: PDOKAN_FILE_INFO,
) -> NTSTATUS {
    wrap_ffi(|| unsafe {
        let dokan_file_info = &mut *dokan_file_info;
        let file = file_from_dokan_file_info(dokan_file_info);
        file.set_delete(true)?;
        Ok(())
    })
}

pub(super) extern "stdcall" fn delete_directory(
    file_name: LPCWSTR,
    dokan_file_info: PDOKAN_FILE_INFO,
) -> NTSTATUS {
    wrap_ffi(|| unsafe {
        let dokan_file_info = &mut *dokan_file_info;
        let file = file_from_dokan_file_info(dokan_file_info);
        file.set_delete(true)?;
        Ok(())
    })
}

pub(super) extern "stdcall" fn move_file(
    file_name: LPCWSTR,
    new_file_name: LPCWSTR,
    replace_if_existing: BOOL,
    dokan_file_info: PDOKAN_FILE_INFO,
) -> NTSTATUS {
    wrap_ffi(|| unsafe {
        let dokan_file_info = &mut *dokan_file_info;
        let file = file_from_dokan_file_info(dokan_file_info);
        let new_path = U16CStr::from_ptr_str(new_file_name);
        let new_path = U16SegPath::new(new_path, PathDelimiter::BackSlash);
        file.wide_move_to(new_path, replace_if_existing != 0)?;
        Ok(())
    })
}

pub(super) extern "stdcall" fn set_end_of_file(
    file_name: LPCWSTR,
    byte_offset: LONGLONG,
    dokan_file_info: PDOKAN_FILE_INFO,
) -> NTSTATUS {
    wrap_ffi(|| unsafe {
        let dokan_file_info = &mut *dokan_file_info;
        let file = file_from_dokan_file_info(dokan_file_info);
        file.set_end_of_file(byte_offset as _)?;
        Ok(())
    })
}

pub(super) extern "stdcall" fn set_allocation_size(
    file_name: LPCWSTR,
    alloc_size: LONGLONG,
    dokan_file_info: PDOKAN_FILE_INFO,
) -> NTSTATUS {
    // TODO: set_allocation_size
    wrap_ffi(|| unsafe {
        let dokan_file_info = &mut *dokan_file_info;
        let file = file_from_dokan_file_info(dokan_file_info);
        // TODO...
        log::debug!("set_allocation_size is stubbed");
        Ok(())
    })
}

pub(super) extern "stdcall" fn get_disk_free_space(
    free_bytes_available: PULONGLONG,
    total_number_of_bytes: PULONGLONG,
    total_number_of_free_bytes: PULONGLONG,
    dokan_file_info: PDOKAN_FILE_INFO,
) -> NTSTATUS {
    wrap_ffi(|| {
        let dokan_file_info = unsafe { &mut *dokan_file_info };
        let server = server_from_dokan_file_info(dokan_file_info);
        let result = server.fs.get_fs_free_space()?;
        unsafe {
            free_bytes_available.write(result.available_bytes_count);
            total_number_of_bytes.write(result.bytes_count);
            total_number_of_free_bytes.write(result.free_bytes_count);
        }
        Ok(())
    })
}

pub(super) extern "stdcall" fn get_volume_information(
    volume_name_buffer: LPWSTR,
    volume_name_size: DWORD,
    volume_serial_number: LPDWORD,
    maximum_component_length: LPDWORD,
    file_system_flags: LPDWORD,
    file_system_name_buffer: LPWSTR,
    file_system_name_size: DWORD,
    dokan_file_info: PDOKAN_FILE_INFO,
) -> NTSTATUS {
    // TODO: get_volume_information
    wrap_ffi(|| Err(FileSystemError::NotImplemented))
}

pub(super) extern "stdcall" fn mounted(
    mount_point: LPCWSTR,
    dokan_file_info: PDOKAN_FILE_INFO,
) -> NTSTATUS {
    wrap_ffi(|| {
        let mount_point = unsafe { U16CStr::from_ptr_str(mount_point) }.to_string_lossy();
        log::info!("Assigned {mount_point} to Dokan drive");
        Ok(())
    })
}

pub(super) extern "stdcall" fn unmounted(dokan_file_info: PDOKAN_FILE_INFO) -> NTSTATUS {
    wrap_ffi(|| {
        log::trace!("Unmounted Dokan drive");

        let dokan_file_info = unsafe { &mut *dokan_file_info };
        let server = server_from_dokan_file_info(dokan_file_info);
        server.shutdown_flag.store(1, Ordering::Release);
        atomic_wait::wake_one(&server.shutdown_flag);
        Ok(())
    })
}
