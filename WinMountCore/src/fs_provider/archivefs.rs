mod zip;

use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    convert::Infallible,
    ptr::NonNull,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use regex::Regex;
use serde::{Deserialize, Serialize};
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

struct ArchiveHandlerWithFilesDepFilesInfo<'a> {
    file: super::OwnedFile<'a>,
    is_dir: bool,
    ref_count: u64,
}
struct ArchiveHandlerWithFilesChildFilesInfo {
    file: OwnedArchiveFile<'static>,
    is_dir: bool,
    ref_count: u64,
}

unsafe impl Send for ArchiveHandlerWithFiles<'_> {}

struct ArchiveHandlerWithFiles<'a> {
    handler: Option<NonNull<dyn ArchiveHandler>>,
    // Files used by handler
    dep_files: Mutex<BTreeMap<CaselessString, ArchiveHandlerWithFilesDepFilesInfo<'a>>>,
    // Files created from handler
    files: Mutex<BTreeMap<CaselessString, ArchiveHandlerWithFilesChildFilesInfo>>,
    // NOTE: base_path is already combined with FsWithPath.path
    base_path: CaselessString,
    path: CaselessString,
}
impl<'a> ArchiveHandlerWithFiles<'a> {
    unsafe fn new(
        handler: Option<NonNull<dyn ArchiveHandler>>,
        base_path: CaselessString,
        filename: CaselessString,
        file: super::OwnedFile<'a>,
        is_dir: bool,
    ) -> Self {
        let mut dep_files = BTreeMap::new();
        dep_files.insert(
            filename.clone(),
            ArchiveHandlerWithFilesDepFilesInfo {
                file,
                is_dir,
                ref_count: 1,
            },
        );
        Self {
            handler,
            dep_files: Mutex::new(dep_files),
            files: Mutex::new(BTreeMap::new()),
            base_path,
            path: filename,
        }
    }
}
impl Drop for ArchiveHandlerWithFiles<'_> {
    fn drop(&mut self) {
        // self.files.lock().unwrap().retain(|_, v| unsafe {
        //     let _ = Box::from_raw(v.file);
        //     false
        // });
        self.files.get_mut().unwrap().clear();
        if let Some(handler) = self.handler.take() {
            let _ = unsafe { Box::from_raw(handler.as_ptr()) };
        }
    }
}

// NOTE: ArchiveFs is primaily read-only
struct ArchiveFsHandler {
    in_path: FsWithPath,
    // TODO: Correctly annonate lifetime of ArchiveHandlerWithFiles
    open_archives: Mutex<BTreeMap<CaselessString, Box<ArchiveHandlerWithFiles<'static>>>>,
    archive_rules: Vec<ArchiveOpenRuleConfig>,
    non_unicode_compat: ArchiveGlobalNonUnicodeCompatConfig,
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
    // TODO: Describe haystack format
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

// NOTE: Honors neither handles_file nor handles_file
fn is_name_archive<'a>(
    name: &str,
    rules: &'a Vec<ArchiveOpenRuleConfig>,
) -> Option<&'a ArchiveOpenRuleConfig> {
    rules
        .iter()
        .filter(|x| x.path_pattern.is_match(name))
        .next()
}

fn split_archive_path<'a, 'b>(
    path: super::SegPath<'b>,
    rules: &'a Vec<ArchiveOpenRuleConfig>,
) -> Option<(
    super::SegPath<'b>,
    super::SegPath<'b>,
    &'a ArchiveOpenRuleConfig,
)> {
    use super::SegPath;
    let delimiter = path.get_delimiter();
    let path = path.get_path();
    path.match_indices(delimiter.as_char())
        .chain(std::iter::once((path.len(), "")))
        .filter_map(|(x, _)| {
            let (front_path, back_path) = path.split_at(x);
            is_name_archive(front_path, rules).map(|x| unsafe {
                // SAFETY: Source is already SegPath
                let front_path = SegPath::new_unchecked(front_path, delimiter);
                let back_path = SegPath::new_unchecked(back_path, delimiter);
                (front_path, back_path, x)
            })
        })
        .next()
    /*
    let mut iter = path.iter();
    while let Some(s) = iter.next() {
        if is_name_archive(s) {
            return Some(iter.into_split());
        }
    }
    None
    */
}

struct ArchiveHandlerOpenContextOpenInfo<'a> {
    context: ArchiveOpenDepFileGuard<'a>,
    is_dir: bool,
}

struct ArchiveOpenDepFileGuard<'a> {
    file: &'a dyn super::File,
    path: CaselessString,
    ctx: &'a ArchiveHandlerWithFiles<'a>,
}
impl<'a> ArchiveOpenDepFileGuard<'a> {
    fn new(
        file: &'a dyn super::File,
        path: CaselessString,
        ctx: &'a ArchiveHandlerWithFiles<'a>,
    ) -> Self {
        Self { file, path, ctx }
    }
}
impl<'a> std::ops::Deref for ArchiveOpenDepFileGuard<'a> {
    type Target = dyn super::File + 'a;
    fn deref(&self) -> &Self::Target {
        self.file
    }
}
impl Drop for ArchiveOpenDepFileGuard<'_> {
    fn drop(&mut self) {
        use std::collections::btree_map::Entry::*;
        let mut dep_files = self.ctx.dep_files.lock().unwrap();
        match dep_files.entry(self.path.clone()) {
            Occupied(mut e) => {
                let info = e.get_mut();
                info.ref_count -= 1;
                if info.ref_count == 0 {
                    e.remove();
                }
            }
            Vacant(_) => {
                panic!("dropping a non-existent archive dep file");
            }
        }
    }
}

#[derive(Clone, Copy)]
struct ArchiveHandlerOpenContext<'a> {
    file: &'a dyn super::File,
    is_dir: bool,
    ctx: &'a ArchiveHandlerWithFiles<'a>,
    fs: &'a FsWithPath,
}
impl<'a> ArchiveHandlerOpenContext<'a> {
    // NOTE: The base path will be automatically added
    fn open_file(
        &self,
        filename: super::SegPath,
    ) -> super::FileSystemResult<ArchiveHandlerOpenContextOpenInfo<'a>> {
        use std::collections::btree_map::Entry::*;
        let filename = super::concat_path(self.ctx.base_path.as_str(), filename);
        let filename = filename.as_non_owned();
        let filename_str: CaselessString = filename.get_path().into();
        let mut dep_files = self.ctx.dep_files.lock().unwrap();
        let info = match dep_files.entry(filename_str.clone()) {
            Occupied(e) => {
                let info = e.into_mut();
                info.ref_count += 1;
                info
            }
            Vacant(e) => {
                let create_info = self.fs.handler.create_file(
                    filename,
                    FileDesiredAccess::Read,
                    FileAttributes::empty(),
                    FileShareAccess::Read,
                    FileCreateDisposition::OpenExisting,
                    FileCreateOptions::empty(),
                )?;
                e.insert(ArchiveHandlerWithFilesDepFilesInfo {
                    file: create_info.context,
                    is_dir: create_info.is_dir,
                    ref_count: 1,
                })
            }
        };
        // TODO: SAFETY statement
        let file = unsafe { std::mem::transmute(&*info.file) };
        Ok(ArchiveHandlerOpenContextOpenInfo {
            context: ArchiveOpenDepFileGuard::new(file, filename_str, self.ctx),
            is_dir: info.is_dir,
        })
    }
    pub fn get_file(&self) -> &'a dyn super::File {
        self.file
    }
    pub fn get_is_dir(&self) -> bool {
        self.is_dir
    }
}

fn open_archive_from_file<'a>(
    open_ctx: ArchiveHandlerOpenContext<'a>,
    archive_rule: &ArchiveOpenRuleConfig,
    non_unicode_compat: &ArchiveNonUnicodeCompatConfig,
) -> anyhow::Result<Box<dyn ArchiveHandler + 'a>> {
    Ok(Box::new(match archive_rule.handler_kind {
        ArchiveHandlerKind::Zip => zip::ZipArchive::new(open_ctx, non_unicode_compat)?,
        _ => anyhow::bail!("unsupported archive handler type"),
    }))
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

        let filename = super::concat_path(&self.in_path.path, filename);
        let filename = filename.as_non_owned();

        const UNWANTED_ACCESS: FileDesiredAccess =
            FileDesiredAccess::Full.union(FileDesiredAccess::Write);

        // TODO: SAFETY statement

        let ensure_file_kind_fn = |is_dir: bool| {
            if create_options.contains(FileCreateOptions::DirectoryFile) && !is_dir {
                return Err(FileSystemError::NotADirectory);
            }
            if create_options.contains(FileCreateOptions::NonDirectoryFile) && is_dir {
                return Err(FileSystemError::FileIsADirectory);
            }
            Ok(())
        };

        let handle_raw_file_fn = || {
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
                context: Box::new(ArchiveFsFile::new_raw(self, raw_create_result.context)),
                is_dir: raw_create_result.is_dir,
                new_file_created: false,
            })
        };

        if let Some((front_path, back_path, archive_rule)) =
            split_archive_path(filename, &self.archive_rules)
        {
            // Handle archive
            let mut entries = self.open_archives.lock().unwrap();

            match entries.entry(front_path.get_path().into()) {
                Occupied(e) => {
                    let entry = e.into_mut();
                    let files = entry.files.get_mut().unwrap();
                    let file_info = match files.entry(back_path.get_path().into()) {
                        Occupied(e) => {
                            // let key: &str = unsafe { std::mem::transmute(e.key().as_str()) };
                            let info = e.into_mut();
                            ensure_file_kind_fn(info.is_dir)?;
                            // log::debug!("File `{}` inc refcnt = {}", key, info.ref_count + 1);
                            info.ref_count += 1;
                            info
                        }
                        Vacant(e) => unsafe {
                            let archive = entry.handler.unwrap_unchecked().as_ref();
                            let open_result = archive.open_file(back_path)?;
                            ensure_file_kind_fn(open_result.is_dir)?;
                            // log::debug!("Adding file `{}` to cache...", e.key().as_str());
                            e.insert(ArchiveHandlerWithFilesChildFilesInfo {
                                file: open_result.context,
                                is_dir: open_result.is_dir,
                                ref_count: 1,
                            })
                        },
                    };
                    let index = front_path.get_path().into();
                    Ok(super::CreateFileInfo {
                        context: unsafe {
                            Box::new(ArchiveFsFile::new_archive(
                                self,
                                index,
                                &self.open_archives,
                                back_path.get_path().into(),
                                file_info.file.as_ref(),
                            ))
                        },
                        is_dir: file_info.is_dir,
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

                    // Check whether rule covers current file kind
                    let rule_covers = if raw_create_result.is_dir {
                        archive_rule.handles_folder
                    } else {
                        archive_rule.handles_file
                    };
                    if !rule_covers {
                        // Treat as raw creation instead
                        drop(raw_create_result);
                        return handle_raw_file_fn();
                    }

                    // TODO: Optimize with further borrowing and avoid allocations
                    let non_unicode_compat = self
                        .non_unicode_compat
                        .entries
                        .iter()
                        .filter(|x| x.path_pattern.is_match(orig_filename.get_path()))
                        .next()
                        .map(|x| Cow::Borrowed(&x.config))
                        .unwrap_or_else(|| {
                            Cow::Owned(ArchiveNonUnicodeCompatConfig {
                                encoding_override: self.non_unicode_compat.encoding.clone(),
                                ..Default::default()
                            })
                        });

                    let mut archive_with_files = unsafe {
                        Box::new(ArchiveHandlerWithFiles::new(
                            None,
                            front_path.get_path().into(),
                            front_path.get_path().into(),
                            raw_create_result.context,
                            raw_create_result.is_dir,
                        ))
                    };
                    let root_file = unsafe {
                        std::mem::transmute(
                            archive_with_files
                                .dep_files
                                .get_mut()
                                .unwrap()
                                .first_key_value()
                                .unwrap()
                                .1
                                .file
                                .as_ref(),
                        )
                    };

                    let open_context = ArchiveHandlerOpenContext {
                        file: root_file,
                        is_dir: raw_create_result.is_dir,
                        ctx: unsafe { std::mem::transmute(archive_with_files.as_ref()) },
                        fs: &self.in_path,
                    };
                    let archive =
                        open_archive_from_file(open_context, archive_rule, &non_unicode_compat)
                            .map_err(|e| {
                                log::warn!("Open archive `{}` failed: {e}", front_path.get_path());
                                FileSystemError::FileCorruptError
                            })?;
                    let archive = unsafe {
                        let archive: Box<dyn ArchiveHandler> = std::mem::transmute(archive);
                        archive_with_files.handler =
                            Some(NonNull::new_unchecked(Box::into_raw(archive)));
                        &*archive_with_files.handler.unwrap_unchecked().as_ptr()
                    };

                    let open_result = archive.open_file(back_path)?;
                    ensure_file_kind_fn(open_result.is_dir)?;

                    // SAFETY: File is behind Box (points to heap)
                    let open_result: ArchiveHandlerOpenFileInfo<'static> =
                        unsafe { std::mem::transmute(open_result) };
                    let open_result_context: *const dyn ArchiveFile = open_result.context.as_ref();
                    let index = front_path.get_path().into();
                    match archive_with_files
                        .files
                        .get_mut()
                        .unwrap()
                        .entry(back_path.get_path().into())
                    {
                        Occupied(_) => {
                            panic!("unexpected file found in empty map");
                        }
                        Vacant(e) => {
                            // log::debug!("Adding file `{}` to cache...", e.key().as_str());
                            e.insert(ArchiveHandlerWithFilesChildFilesInfo {
                                file: open_result.context,
                                is_dir: open_result.is_dir,
                                ref_count: 1,
                            });
                        }
                    }
                    // log::debug!("Adding archive `{}` to cache...", front_path.get_path());
                    e.insert(unsafe { std::mem::transmute(archive_with_files) });
                    let context = unsafe {
                        Box::new(ArchiveFsFile::new_archive(
                            self,
                            index,
                            &self.open_archives,
                            back_path.get_path().into(),
                            open_result_context,
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
            handle_raw_file_fn()
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
        entries: &'a Mutex<BTreeMap<CaselessString, Box<ArchiveHandlerWithFiles<'static>>>>,
        filename: CaselessString,
        // TODO: Correctly annotate lifetime of ArchiveFile
        file: *const dyn ArchiveFile,
    },
}

struct ArchiveFsFile<'a, 'h: 'a> {
    handler: &'h ArchiveFsHandler,
    context: ArchiveFsFileContext<'a>,
}

unsafe impl Send for ArchiveFsFile<'_, '_> {}
unsafe impl Sync for ArchiveFsFile<'_, '_> {}

impl<'a, 'h: 'a> ArchiveFsFile<'a, 'h> {
    fn new_raw(handler: &'h ArchiveFsHandler, file: super::OwnedFile<'a>) -> Self {
        ArchiveFsFile {
            handler,
            context: ArchiveFsFileContext::Raw(file),
        }
    }
    // WARN: Do NOT drop ArchiveHandlerWithFiles outside, this is fully covered by
    //       ArchiveFsFile
    // WARN: Insert into entries right before calling this method!
    unsafe fn new_archive(
        handler: &'h ArchiveFsHandler,
        index: CaselessString,
        entries: &'a Mutex<BTreeMap<CaselessString, Box<ArchiveHandlerWithFiles<'static>>>>,
        filename: CaselessString,
        file: *const dyn ArchiveFile,
    ) -> Self {
        ArchiveFsFile {
            handler,
            context: ArchiveFsFileContext::Archive {
                index,
                entries,
                filename,
                file,
            },
        }
    }
}

impl Drop for ArchiveFsFile<'_, '_> {
    fn drop(&mut self) {
        use std::collections::btree_map::Entry::*;

        if let ArchiveFsFileContext::Archive {
            index,
            entries,
            filename,
            file,
        } = &mut self.context
        {
            // Check if we need to remove entry
            let mut entries = entries.lock().unwrap();
            match entries.entry(index.clone()) {
                Occupied(mut e) => {
                    let entry = e.get_mut();
                    let files = entry.files.get_mut().unwrap();
                    match files.entry(filename.clone()) {
                        Occupied(mut e) => {
                            let info = e.get_mut();
                            // log::debug!("File `{}` dec refcnt = {}", filename.as_str(), info.ref_count - 1);
                            info.ref_count -= 1;
                            if info.ref_count == 0 {
                                // Drop file
                                // log::debug!("Removing file `{}` from cache...", filename.as_str());
                                e.remove();
                            }
                        }
                        Vacant(_) => {
                            panic!("file missing in open files list");
                        }
                    }
                    if files.is_empty() {
                        // The last file from the archive has been closed, remove the
                        // archive entry now
                        // log::debug!("Removing archive `{}` from cache...", e.key().as_str());
                        e.remove();
                    }
                }
                Vacant(_) => panic!("archive entry missing while inner file is open"),
            }
        }
    }
}

impl super::File for ArchiveFsFile<'_, '_> {
    fn read_at(&self, offset: u64, buffer: &mut [u8]) -> super::FileSystemResult<u64> {
        match &self.context {
            ArchiveFsFileContext::Raw(f) => f.read_at(offset, buffer),
            ArchiveFsFileContext::Archive { file, .. } => {
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
            ArchiveFsFileContext::Archive { file, .. } => {
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
                struct ArchiveFsFiller<'a, 'b, 'h> {
                    this: &'a ArchiveFsFile<'b, 'h>,
                    filler: &'a mut dyn super::FindFilesDataFiller,
                }
                impl super::FindFilesDataFiller for ArchiveFsFiller<'_, '_, '_> {
                    fn fill_data(
                        &mut self,
                        name: &str,
                        stat: &super::FileStatInfo,
                    ) -> Result<(), ()> {
                        if stat.is_dir {
                            return self.filler.fill_data(name, stat);
                        }
                        let check_file_kind_fn = |rule: &ArchiveOpenRuleConfig| {
                            if stat.is_dir {
                                rule.handles_folder
                            } else {
                                rule.handles_file
                            }
                        };
                        match is_name_archive(name, &self.this.handler.archive_rules) {
                            Some(rule) if check_file_kind_fn(rule) => {
                                // We need to transform files into folders here
                                let mut stat = *stat;
                                stat.is_dir = true;
                                stat.attributes |= FileAttributes::DirectoryFile;
                                stat.size = 0;
                                self.filler.fill_data(name, &stat)
                            }
                            _ => self.filler.fill_data(name, stat),
                        }
                    }
                }
                f.find_files_with_pattern(pattern, &mut ArchiveFsFiller { this: self, filler })
            }
            ArchiveFsFileContext::Archive { file, .. } => {
                let file = unsafe { &**file };
                file.find_files_with_pattern(pattern, filler)
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum ArchiveHandlerKind {
    Zip,
}

#[derive(Serialize, Deserialize, Debug)]
struct ArchiveOpenRuleConfig {
    // TODO: Describe haystack format
    // WARN: Should only be used to check against file suffix;
    //       won't be applied if path is already a folder
    #[serde(with = "serde_regex")]
    path_pattern: Regex,
    handler_kind: ArchiveHandlerKind,
    handles_file: bool,
    handles_folder: bool,
}

#[derive(Serialize, Deserialize)]
struct ArchiveFsConfig {
    /// The input path (can be an archive file or a folder containing archive files).
    input_path: FsWithPathConfig,
    /// Rules to filter files to be treated as archives.
    archive_rules: Vec<ArchiveOpenRuleConfig>,
    /// Non-Unicode compatibility configurations for archives.
    non_unicode_compat: ArchiveGlobalNonUnicodeCompatConfig,
}

impl ArchiveFsHandler {
    fn new(
        in_path: FsWithPath,
        archive_rules: Vec<ArchiveOpenRuleConfig>,
        non_unicode_compat: ArchiveGlobalNonUnicodeCompatConfig,
    ) -> Self {
        ArchiveFsHandler {
            in_path,
            open_archives: Mutex::new(BTreeMap::new()),
            archive_rules,
            non_unicode_compat,
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
        super::make_uniform_path(&mut config.input_path.path);
        let in_path = FsWithPath {
            handler: ctx.get_or_run_fs(&config.input_path.id, "")?,
            path: config.input_path.path,
        };
        Ok(Arc::new(ArchiveFsHandler::new(
            in_path,
            config.archive_rules,
            config.non_unicode_compat,
        )))
    }
    fn get_template_config(&self) -> serde_json::Value {
        serde_json::to_value(ArchiveFsConfig {
            input_path: Default::default(),
            archive_rules: vec![ArchiveOpenRuleConfig {
                path_pattern: Regex::new(r"\.(?i)zip$").unwrap(),
                handler_kind: ArchiveHandlerKind::Zip,
                handles_file: true,
                handles_folder: false,
            }],
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
