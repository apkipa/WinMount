mod zip;

use std::{
    borrow::Cow,
    collections::BTreeMap,
    convert::Infallible,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use uuid::{uuid, Uuid};

use crate::util::{CaselessStr, CaselessString};

use super::{
    FileAttributes, FileCreateDisposition, FileCreateOptions, FileDesiredAccess, FileShareAccess,
    FileSystemError, FsWithPath, FsWithPathConfig,
};

pub const ARCHIVEFS_ID: Uuid = uuid!("65A95C07-AF76-4AD1-B49B-C850581FB87A");

trait ArchiveFile {
    fn read_at(&self, offset: u64, buffer: &mut [u8]) -> super::FileSystemResult<u64>;
    fn get_stat(&self) -> super::FileSystemResult<super::FileStatInfo>;
    fn find_files_with_pattern(
        &self,
        pattern: &dyn super::FilePattern,
        filler: &mut dyn super::FindFilesDataFiller,
    ) -> super::FileSystemResult<()>;
}

type OwnedArchiveFile<'a> = Box<dyn ArchiveFile + 'a>;

struct ArchiveHandlerOpenFileInfo<'a> {
    context: OwnedArchiveFile<'a>,
    is_dir: bool,
}

trait ArchiveHandler: Send {
    fn open_file(
        &self,
        filename: super::SegPath,
    ) -> super::FileSystemResult<ArchiveHandlerOpenFileInfo<'_>>;
}

struct ArchiveHandlerWithFiles<'a> {
    handler: Box<dyn ArchiveHandler + 'a>,
    // files: Vec<OwnedArchiveFile>,
    // TODO: Do we really need AtomicU64 here?
    open_count: AtomicU64,
}

// NOTE: ArchiveFs is primaily read-only
struct ArchiveFsHandler {
    in_path: FsWithPath,
    // TODO: Correctly annonate lifetime of ArchiveHandlerWithFiles
    open_archives: Mutex<BTreeMap<CaselessString, ArchiveHandlerWithFiles<'static>>>,
    non_unicode_compat_cfg: ArchiveGlobalNonUnicodeCompatConfig,
}

#[derive(Clone)]
enum ArchiveNonUnicodeEncoding {
    System,
    AutoDetect,
    Specified(String),
}
impl Serialize for ArchiveNonUnicodeEncoding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match *self {
            ArchiveNonUnicodeEncoding::System => "",
            ArchiveNonUnicodeEncoding::AutoDetect => "auto",
            ArchiveNonUnicodeEncoding::Specified(ref lang) => lang,
        })
    }
}
impl<'de> Deserialize<'de> for ArchiveNonUnicodeEncoding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_ascii_lowercase().as_ref() {
            "" => ArchiveNonUnicodeEncoding::System,
            "auto" => ArchiveNonUnicodeEncoding::AutoDetect,
            _ => ArchiveNonUnicodeEncoding::Specified(s),
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct ArchiveNonUnicodeCompatConfig {
    encoding_override: ArchiveNonUnicodeEncoding,
    /// Whether to treat valid-UTF-8-sequence names as UTF-8.
    allow_utf8_mix: bool,
    /// Whether to ignore UTF-8 markers on names.
    ignore_utf8_flags: bool,
}
impl Default for ArchiveNonUnicodeCompatConfig {
    fn default() -> Self {
        Self {
            encoding_override: ArchiveNonUnicodeEncoding::System,
            allow_utf8_mix: false,
            ignore_utf8_flags: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct ArchiveNonUnicodeCompatConfigEntry {
    #[serde(with = "serde_regex")]
    path_pattern: Regex,
    #[serde(flatten)]
    config: ArchiveNonUnicodeCompatConfig,
}

#[derive(Serialize, Deserialize, Clone)]
struct ArchiveGlobalNonUnicodeCompatConfig {
    encoding: ArchiveNonUnicodeEncoding,
    entries: Vec<ArchiveNonUnicodeCompatConfigEntry>,
}

impl Default for ArchiveGlobalNonUnicodeCompatConfig {
    fn default() -> Self {
        Self {
            encoding: ArchiveNonUnicodeEncoding::System,
            entries: Vec::new(),
        }
    }
}

enum EncodingConverterEngine {
    Win32MBWC { from_cp: u32, to_cp: u32 },
    Win32ICU(Infallible),
    RustEncoding(&'static encoding_rs::Encoding),
}

struct EncodingConverter {
    engine: EncodingConverterEngine,
}

impl EncodingConverter {
    fn new(encoding: &ArchiveNonUnicodeEncoding) -> Self {
        use windows::Win32::Globalization::*;

        if matches!(encoding, ArchiveNonUnicodeEncoding::AutoDetect) {
            panic!("AutoDetect is not a valid encoding for EncodingConverter")
        }
        let engine = match encoding {
            ArchiveNonUnicodeEncoding::System => EncodingConverterEngine::Win32MBWC {
                from_cp: CP_ACP,
                to_cp: CP_UTF8,
            },
            ArchiveNonUnicodeEncoding::AutoDetect => {
                panic!("AutoDetect is not a valid encoding for EncodingConverter");
            }
            ArchiveNonUnicodeEncoding::Specified(s) => {
                let encoding =
                    encoding_rs::Encoding::for_label(s.as_bytes()).expect("invalid encoding name");
                EncodingConverterEngine::RustEncoding(encoding)
            }
        };
        Self { engine }
    }
    fn convert<'a>(&self, bytes: &'a [u8]) -> Cow<'a, str> {
        use windows::Win32::Globalization::*;

        match self.engine {
            EncodingConverterEngine::Win32MBWC { from_cp, to_cp } => unsafe {
                let wide_buf = {
                    let size =
                        MultiByteToWideChar(from_cp, MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0), bytes, None);
                    if size == 0 {
                        panic!("MultiByteToWideChar returned 0");
                    }
                    let mut wide_buf = Vec::with_capacity(size as _);
                    // TODO: Is it safe to form a mut slice into uninitialized memory?
                    //       (https://github.com/rust-lang/miri/issues/1240)
                    let size2 = MultiByteToWideChar(
                        from_cp,
                        MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0),
                        bytes,
                        Some(std::slice::from_raw_parts_mut(
                            wide_buf.as_mut_ptr(),
                            size as _,
                        )),
                    );
                    if size != size2 {
                        panic!("MultiByteToWideChar returned different size");
                    }
                    wide_buf.set_len(size as _);
                    wide_buf
                };
                {
                    let size = WideCharToMultiByte(to_cp, 0, &wide_buf, None, None, None);
                    if size == 0 {
                        panic!("WideCharToMultiByte returned 0");
                    }
                    let mut buf = Vec::with_capacity(size as _);
                    let size2 = WideCharToMultiByte(
                        to_cp,
                        0,
                        &wide_buf,
                        Some(std::slice::from_raw_parts_mut(buf.as_mut_ptr(), size as _)),
                        None,
                        None,
                    );
                    if size != size2 {
                        panic!("WideCharToMultiByte returned different size");
                    }
                    buf.set_len(size as _);
                    Cow::Owned(String::from_utf8_unchecked(buf))
                }
            },
            EncodingConverterEngine::RustEncoding(encoding) => {
                encoding.decode_without_bom_handling(bytes).0
            }
            _ => todo!("not implemented"),
        }
    }
}

// NOTE: Result path will be `\`-separated
// WARN: Input must be valid
fn concat_path(base: &str, path: super::SegPath) -> super::OwnedSegPath {
    // let path = if base.is_empty() {
    //     path.get_path().to_owned()
    // } else {
    //     format!("{}\\{}", base, path.get_path())
    // };
    let path = if let super::PathDelimiter::BackSlash = path.get_delimiter() {
        format!("{}\\{}", base, path.get_path())
    } else {
        let mut path = format!("{}\\{}", base, path.get_path());
        make_uniform_path(&mut path);
        path
    };
    super::OwnedSegPath::new(path, super::PathDelimiter::BackSlash)
}

const ARCHIVE_PATTERNS: &[&str] = &[".zip"];

fn is_name_archive(name: &str) -> bool {
    ARCHIVE_PATTERNS.iter().any(|x| name.ends_with(x))
}

// TODO: Support user-defined archive name filters
fn split_archive_path(path: super::SegPath) -> Option<(super::SegPath, super::SegPath)> {
    use super::SegPath;
    let delimiter = path.get_delimiter();
    let path = path.get_path();
    path.match_indices(delimiter.as_char())
        .chain(std::iter::once((path.len(), "")))
        .filter_map(|(x, _)| {
            let (front_path, back_path) = path.split_at(x);
            is_name_archive(front_path).then_some(unsafe {
                // SAFETY: Source is already SegPath
                let front_path = SegPath::new_unchecked(front_path, delimiter);
                let back_path = SegPath::new_unchecked(back_path, delimiter);
                (front_path, back_path)
            })
        })
        .next()
}

fn open_archive_from_file<'a>(
    file: super::OwnedFile<'a>,
    non_unicode_compat: &ArchiveNonUnicodeCompatConfig,
) -> anyhow::Result<Box<dyn ArchiveHandler + 'a>> {
    // TODO: open_archive_from_file
    // anyhow::bail!("unimplemented")
    // TODO: Check header (ArchiveHandler should have a check method returning
    //       Yes, No, Maybe(?))
    match zip::ZipArchive::new(file, non_unicode_compat) {
        Ok(x) => Ok(Box::new(x)),
        Err((_, e)) => Err(e.into()),
    }
}

impl super::FileSystemHandler for ArchiveFsHandler {
    fn create_file(
        &self,
        filename: super::SegPath,
        desired_access: FileDesiredAccess,
        file_attributes: FileAttributes,
        share_access: FileShareAccess,
        create_disposition: FileCreateDisposition,
        create_options: FileCreateOptions,
    ) -> super::FileSystemResult<super::CreateFileInfo<'_>> {
        use std::collections::btree_map::Entry::*;

        let orig_filename = filename;

        let filename = concat_path(&self.in_path.path, filename);
        let filename = filename.as_non_owned();

        const UNWANTED_ACCESS: FileDesiredAccess =
            FileDesiredAccess::Full.union(FileDesiredAccess::Write);

        if let Some((front_path, back_path)) = split_archive_path(filename) {
            // Handle archive

            // log::debug!("archivefs: Opening `{}` / `{}`...", front_path.get_path(), back_path.get_path());

            let mut entries = self.open_archives.lock().unwrap();

            match entries.entry(CaselessString::new(front_path.get_path().to_string())) {
                Occupied(e) => {
                    let entry = e.get();
                    // TODO: Maybe we can share opened file instances?
                    let open_result = entry.handler.open_file(back_path)?;
                    let index = e.key().clone();
                    entry.open_count.fetch_add(1, Ordering::AcqRel);
                    Ok(super::CreateFileInfo {
                        context: unsafe {
                            Box::new(ArchiveFsFile::new_archive(
                                index,
                                &self.open_archives,
                                open_result.context,
                            ))
                        },
                        is_dir: open_result.is_dir,
                        new_file_created: false,
                    })
                }
                Vacant(e) => {
                    let raw_create_result = self.in_path.handler.create_file(
                        front_path,
                        FileDesiredAccess::Read,
                        FileAttributes::empty(),
                        FileShareAccess::Read,
                        FileCreateDisposition::OpenExisting,
                        FileCreateOptions::empty(),
                    )?;
                    if raw_create_result.is_dir {
                        // TODO: This is actually a folder, do not handle as an archive
                        return Err(FileSystemError::NotImplemented);
                    }

                    // TODO: Optimize with further borrowing and avoid allocations
                    let non_unicode_compat_cfg = self
                        .non_unicode_compat_cfg
                        .entries
                        .iter()
                        .filter(|x| x.path_pattern.is_match(orig_filename.get_path()))
                        .next()
                        .map(|x| Cow::Borrowed(&x.config))
                        .unwrap_or_else(|| {
                            Cow::Owned(ArchiveNonUnicodeCompatConfig {
                                encoding_override: self.non_unicode_compat_cfg.encoding.clone(),
                                ..Default::default()
                            })
                        });

                    let archive =
                        open_archive_from_file(raw_create_result.context, &non_unicode_compat_cfg)
                            .map_err(|e| {
                                log::warn!("Open archive {} failed: {e}", front_path.get_path());
                                FileSystemError::FileCorruptError
                            })?;
                    let archive: Box<dyn ArchiveHandler> = unsafe { std::mem::transmute(archive) };

                    let open_result = archive.open_file(back_path)?;
                    // TODO: Check for non-dir / dir flags

                    // SAFETY: File is behind Box (points to heap)
                    let open_result: ArchiveHandlerOpenFileInfo<'static> =
                        unsafe { std::mem::transmute(open_result) };
                    let index = CaselessString::new(front_path.get_path().to_string());
                    e.insert(ArchiveHandlerWithFiles {
                        handler: archive,
                        open_count: AtomicU64::new(1),
                    });
                    let context = unsafe {
                        Box::new(ArchiveFsFile::new_archive(
                            index,
                            &self.open_archives,
                            open_result.context,
                        ))
                    };

                    Ok(super::CreateFileInfo {
                        context,
                        is_dir: open_result.is_dir,
                        new_file_created: false,
                    })
                }
            }
        } else {
            // Open raw file
            // NOTE: We must open the file first to make sure file exists
            let raw_create_result = self.in_path.handler.create_file(
                filename,
                FileDesiredAccess::Read,
                FileAttributes::empty(),
                FileShareAccess::Read,
                FileCreateDisposition::OpenExisting,
                create_options,
            )?;

            if desired_access.intersects(UNWANTED_ACCESS) {
                return Err(FileSystemError::AccessDenied);
            }
            Ok(super::CreateFileInfo {
                context: Box::new(ArchiveFsFile::new_raw(raw_create_result.context)),
                is_dir: raw_create_result.is_dir,
                new_file_created: false,
            })
        }
    }
    fn get_fs_free_space(&self) -> super::FileSystemResult<super::FileSystemSpaceInfo> {
        self.in_path.handler.get_fs_free_space()
    }
    fn get_fs_characteristics(&self) -> super::FileSystemResult<super::FileSystemCharacteristics> {
        let mut chars = self.in_path.handler.get_fs_characteristics()?;
        chars |= super::FileSystemCharacteristics::ReadOnly;
        Ok(chars)
    }
}

impl Drop for ArchiveFsHandler {
    fn drop(&mut self) {
        // Avoid lifetime issues
        self.open_archives.lock().unwrap().clear();
    }
}

enum ArchiveFsFileContext<'a> {
    Raw(super::OwnedFile<'a>),
    Archive {
        index: CaselessString,
        entries: &'a Mutex<BTreeMap<CaselessString, ArchiveHandlerWithFiles<'static>>>,
        // TODO: Correctly annotate lifetime of ArchiveFile
        file: *mut dyn ArchiveFile,
    },
}

struct ArchiveFsFile<'a> {
    context: ArchiveFsFileContext<'a>,
}

unsafe impl Send for ArchiveFsFile<'_> {}
unsafe impl Sync for ArchiveFsFile<'_> {}

impl<'a> ArchiveFsFile<'a> {
    fn new_raw(file: super::OwnedFile<'a>) -> Self {
        ArchiveFsFile {
            context: ArchiveFsFileContext::Raw(file),
        }
    }
    // WARN: Do NOT drop ArchiveHandlerWithFiles outside, this is fully covered by
    //       ArchiveFsFile
    // WARN: Add counter right before calling this method!
    unsafe fn new_archive<'b>(
        index: CaselessString,
        entries: &'a Mutex<BTreeMap<CaselessString, ArchiveHandlerWithFiles<'static>>>,
        file: OwnedArchiveFile<'b>,
    ) -> Self
    where
        'a: 'b,
    {
        ArchiveFsFile {
            context: ArchiveFsFileContext::Archive {
                index,
                entries,
                file: std::mem::transmute(Box::into_raw(file)),
            },
        }
    }
}

impl Drop for ArchiveFsFile<'_> {
    fn drop(&mut self) {
        use std::collections::btree_map::Entry::*;

        if let ArchiveFsFileContext::Archive {
            index,
            entries,
            file,
        } = &mut self.context
        {
            // Drop file first
            let _ = unsafe { Box::from_raw(*file) };
            // Then check if we need to remove entry
            let mut entries = entries.lock().unwrap();
            match entries.entry(index.clone()) {
                Occupied(e) => {
                    let entry = e.get();
                    if entry.open_count.fetch_sub(1, Ordering::AcqRel) == 1 {
                        // The last file from the archive has been closed, remove the
                        // archive entry now
                        e.remove();
                    }
                }
                Vacant(_) => panic!("archive entry missing while inner file is open"),
            }
        }
    }
}

impl super::File for ArchiveFsFile<'_> {
    fn read_at(&self, offset: u64, buffer: &mut [u8]) -> super::FileSystemResult<u64> {
        match &self.context {
            ArchiveFsFileContext::Raw(f) => f.read_at(offset, buffer),
            ArchiveFsFileContext::Archive {
                index: _,
                entries: _,
                file,
            } => {
                let file = unsafe { &**file };
                file.read_at(offset, buffer)
            }
        }
    }
    fn write_at(
        &self,
        _offset: Option<u64>,
        _buffer: &[u8],
        _constrain_size: bool,
    ) -> super::FileSystemResult<u64> {
        Err(FileSystemError::AccessDenied)
    }
    fn flush_buffers(&self) -> super::FileSystemResult<()> {
        Err(FileSystemError::AccessDenied)
    }
    fn get_stat(&self) -> super::FileSystemResult<super::FileStatInfo> {
        match &self.context {
            ArchiveFsFileContext::Raw(f) => f.get_stat(),
            ArchiveFsFileContext::Archive {
                index: _,
                entries: _,
                file,
            } => {
                let file = unsafe { &**file };
                file.get_stat()
            }
        }
    }
    fn set_end_of_file(&self, _offset: u64) -> super::FileSystemResult<()> {
        Err(FileSystemError::AccessDenied)
    }
    fn set_file_times(
        &self,
        _creation_time: std::time::SystemTime,
        _last_access_time: std::time::SystemTime,
        _last_write_time: std::time::SystemTime,
    ) -> super::FileSystemResult<()> {
        Err(FileSystemError::AccessDenied)
    }
    fn set_delete(&self, _delete_on_close: bool) -> super::FileSystemResult<()> {
        Err(FileSystemError::AccessDenied)
    }
    fn move_to(
        &self,
        _new_path: super::SegPath,
        _replace_if_exists: bool,
    ) -> super::FileSystemResult<()> {
        Err(FileSystemError::AccessDenied)
    }
    fn find_files_with_pattern(
        &self,
        pattern: &dyn super::FilePattern,
        filler: &mut dyn super::FindFilesDataFiller,
    ) -> super::FileSystemResult<()> {
        match &self.context {
            ArchiveFsFileContext::Raw(f) => {
                struct ArchiveFsFiller<'a> {
                    filler: &'a mut dyn super::FindFilesDataFiller,
                }
                impl super::FindFilesDataFiller for ArchiveFsFiller<'_> {
                    fn fill_data(
                        &mut self,
                        name: &str,
                        stat: &super::FileStatInfo,
                    ) -> Result<(), ()> {
                        if is_name_archive(name) {
                            let mut stat = *stat;
                            stat.is_dir = true;
                            stat.attributes |= FileAttributes::DirectoryFile;
                            stat.size = 0;
                            self.filler.fill_data(name, &stat)
                        } else {
                            self.filler.fill_data(name, stat)
                        }
                    }
                }
                f.find_files_with_pattern(pattern, &mut ArchiveFsFiller { filler })
            }
            ArchiveFsFileContext::Archive {
                index: _,
                entries: _,
                file,
            } => {
                let file = unsafe { &**file };
                file.find_files_with_pattern(pattern, filler)
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ArchiveFsConfig {
    /// The input path (can be an archive file or a folder containing archive files).
    input_path: FsWithPathConfig,
    /// Non-Unicode compatibility configurations for archives.
    non_unicode_compat: ArchiveGlobalNonUnicodeCompatConfig,
}

impl ArchiveFsHandler {
    fn new(
        in_path: FsWithPath,
        non_unicode_compat_cfg: ArchiveGlobalNonUnicodeCompatConfig,
    ) -> Self {
        ArchiveFsHandler {
            in_path,
            open_archives: Mutex::new(BTreeMap::new()),
            non_unicode_compat_cfg,
        }
    }
}

fn make_uniform_path(path: &mut str) {
    // SAFETY: The result string is still valid UTF-8
    for b in unsafe { path.as_bytes_mut() } {
        if *b == '/' as u8 {
            *b = '\\' as u8;
        }
    }
}

pub struct ArchiveFsProvider {}
impl super::FsProvider for ArchiveFsProvider {
    fn get_id(&self) -> Uuid {
        ARCHIVEFS_ID
    }
    fn get_name(&self) -> &'static str {
        "ArchiveFS"
    }
    fn get_version(&self) -> (u32, u32, u32) {
        (0, 1, 0)
    }
    fn construct(
        &self,
        config: serde_json::Value,
        ctx: &mut dyn super::FileSystemCreationContext,
    ) -> Result<std::sync::Arc<dyn super::FileSystemHandler>, super::FileSystemCreationError> {
        let mut config: ArchiveFsConfig = serde_json::from_value(config)
            .map_err(|e| super::FileSystemCreationError::Other(e.into()))?;
        // Translate slashes
        make_uniform_path(&mut config.input_path.path);
        let in_path = FsWithPath {
            handler: ctx.get_or_run_fs(&config.input_path.id, "")?,
            path: config.input_path.path,
        };
        Ok(Arc::new(ArchiveFsHandler::new(
            in_path,
            config.non_unicode_compat,
        )))
    }
    fn get_template_config(&self) -> serde_json::Value {
        serde_json::to_value(ArchiveFsConfig {
            input_path: Default::default(),
            non_unicode_compat: Default::default(),
        })
        .unwrap()
    }
}

impl ArchiveFsProvider {
    pub fn new() -> Self {
        ArchiveFsProvider {}
    }
}
