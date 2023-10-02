use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    io::{Cursor, Read, Seek},
    mem::ManuallyDrop,
    sync::{Mutex, RwLock},
    time::SystemTime,
};

use anyhow::Context;
use binrw::BinRead;

use crate::{
    fs_provider::{
        CursorFile, FileAttributes, FileStatInfo, FileSystemError, FileSystemResult, PathDelimiter,
        SegPath,
    },
    util::{calculate_hash, CaselessStr, CaselessString, SeekExt},
};

use super::{ArchiveHandlerOpenContext, ArchiveNonUnicodeCompatConfig};

mod types;

#[derive(Debug, PartialEq, Eq)]
enum UnityFileType {
    BundleFile,
    WebFile,
    GZipFile,
    BrotliFile,
    AssetsFile,
    ZipFile,
    ResourceFile,
}

fn guess_file_type(file: &mut (impl Read + Seek)) -> binrw::BinResult<UnityFileType> {
    let mut buf = vec![0; 48];
    let read_len = crate::util::read_up_to(file, &mut buf)?;
    buf.truncate(read_len);
    let buf_nullstr = buf
        .iter()
        .position(|&x| x == b'\0')
        .map(|pos| &buf[..pos])
        .unwrap_or(&buf);
    let whole_file_size = file.seek(std::io::SeekFrom::End(0))?;
    file.seek(std::io::SeekFrom::Start(0))?;

    let is_serialized_file_fn = |buf: &[u8]| match || -> anyhow::Result<bool> {
        let len = buf.len();
        let mut buf = Cursor::new(buf);
        if len < 20 {
            return Ok(false);
        }
        let mut metadata_size = u32::read_be(&mut buf)?;
        let mut file_size: u64 = u32::read_be(&mut buf)? as _;
        let version = u32::read_be(&mut buf)?;
        let mut data_offset: u64 = u32::read_be(&mut buf)? as _;
        let endianness = u8::read_be(&mut buf)?;
        let reserved = buf.read_exact(&mut [0; 3])?;
        if version >= 22 {
            if len < 48 {
                return Ok(false);
            }
            metadata_size = u32::read_be(&mut buf)?;
            file_size = u64::read_be(&mut buf)?;
            data_offset = u64::read_be(&mut buf)?;
        }
        Ok(file_size == whole_file_size && data_offset <= file_size)
    }() {
        Ok(x) => x,
        _ => false,
    };

    Ok(match buf_nullstr {
        b"UnityWeb" | b"UnityRaw" | b"UnityArchive" | b"UnityFS" => UnityFileType::BundleFile,
        b"UnityWebData1.0" => UnityFileType::WebFile,
        _ => {
            const GZIP_MAGIC: &[u8] = &[0x1f, 0x8b];
            const BROTLI_MAGIC: &[u8] = &[0x62, 0x72, 0x6f, 0x74, 0x6c, 0x69];
            const ZIP_MAGIC: &[u8] = &[0x50, 0x4b, 0x03, 0x04];
            const ZIP_SPANNED_MAGIC: &[u8] = &[0x50, 0x4b, 0x07, 0x08];
            if buf.starts_with(GZIP_MAGIC) {
                UnityFileType::GZipFile
            } else if buf.len() > 0x20 && buf[0x20..].starts_with(BROTLI_MAGIC) {
                UnityFileType::BrotliFile
            } else if is_serialized_file_fn(&buf) {
                UnityFileType::AssetsFile
            } else if buf.starts_with(ZIP_MAGIC) || buf.starts_with(ZIP_SPANNED_MAGIC) {
                UnityFileType::ZipFile
            } else {
                UnityFileType::ResourceFile
            }
        }
    })
}

#[derive(Clone)]
struct UnityFilePtr {
    base_name: CaselessString,
    // Non-empty if resides in an AB
    inner_offset: Option<SimpleFilePtr>,
    offset: u64,
    size: u64,
}

#[derive(Clone, Copy)]
struct SimpleFilePtr {
    offset: u64,
    size: u64,
}

struct UnityFolderEntry {
    children: BTreeMap<CaselessString, UnityEntry>,
    index: u64,
}
// TODO: Fix get_file_stat_info for UnityFolderEntry & UnityFileEntry
impl UnityFolderEntry {
    fn get_file_stat_info(&self) -> FileStatInfo {
        FileStatInfo {
            index: 0,
            size: 0,
            is_dir: true,
            attributes: FileAttributes::DirectoryFile,
            creation_time: SystemTime::UNIX_EPOCH,
            last_access_time: SystemTime::UNIX_EPOCH,
            last_write_time: SystemTime::UNIX_EPOCH,
        }
    }
}

struct UnityFileEntry {
    // obj_info: types::UnityFSSerializedObjectInfo,
    obj: types::UnityFSSerializedObject,
    file_ptr: UnityFilePtr,
    index: u64,
}
impl UnityFileEntry {
    fn get_file_stat_info(&self) -> FileStatInfo {
        FileStatInfo {
            index: 0,
            size: 0,
            is_dir: false,
            attributes: FileAttributes::empty(),
            creation_time: SystemTime::UNIX_EPOCH,
            last_access_time: SystemTime::UNIX_EPOCH,
            last_write_time: SystemTime::UNIX_EPOCH,
        }
    }
}

enum UnityEntry {
    Folder(UnityFolderEntry),
    File(UnityFileEntry),
}

impl UnityEntry {
    fn is_dir(&self) -> bool {
        matches!(self, Self::Folder(_))
    }
    fn is_file(&self) -> bool {
        matches!(self, Self::File(_))
    }
    fn as_borrowed(&self) -> BorrowedUnityEntry {
        match self {
            Self::Folder(e) => BorrowedUnityEntry::Folder(e),
            Self::File(e) => BorrowedUnityEntry::File(e),
        }
    }
    fn get_file_stat_info(&self) -> FileStatInfo {
        self.as_borrowed().get_file_stat_info()
    }
}

#[derive(Clone, Copy)]
enum BorrowedUnityEntry<'a> {
    Folder(&'a UnityFolderEntry),
    File(&'a UnityFileEntry),
}

impl BorrowedUnityEntry<'_> {
    fn is_dir(&self) -> bool {
        matches!(self, Self::Folder(_))
    }
    fn is_file(&self) -> bool {
        matches!(self, Self::File(_))
    }
    fn get_file_stat_info(&self) -> FileStatInfo {
        match self {
            Self::Folder(e) => e.get_file_stat_info(),
            Self::File(e) => e.get_file_stat_info(),
        }
    }
}

pub struct UnityArchive<'a> {
    open_ctx: ArchiveHandlerOpenContext<'a>,
    root_dir: UnityFolderEntry,
}

// TODO: Remove this
// fn log_err_and_corrupt<T: std::fmt::Display>(e: T) -> FileSystemError {
//     log::warn!("Cannot parse UnityFS: {e}");
//     FileSystemError::FileCorruptError
// }

fn decompress_data(
    compression_type: types::UnityFSAssetCompressionType,
    compressed_data: Vec<u8>,
    uncompressed_size: usize,
) -> anyhow::Result<Vec<u8>> {
    use types::UnityFSAssetCompressionType;
    Ok(match compression_type {
        UnityFSAssetCompressionType::None => compressed_data,
        UnityFSAssetCompressionType::Lzma => {
            // TODO: UnityFSAssetCompressionType::Lzma
            anyhow::bail!("compression type Lzma is not implemented");
        }
        UnityFSAssetCompressionType::Lz4 | UnityFSAssetCompressionType::Lz4HC => {
            lz4_flex::decompress(&compressed_data, uncompressed_size)?
        }
        _ => anyhow::bail!("unsupported compression type {compression_type:?}"),
    })
}

#[derive(Clone, Copy)]
struct UnityBlocksReaderPos {
    raw_pos: u64,
    uncomp_pos: u64,
}

trait ReadAt {
    fn read_at(&self, offset: u64, buffer: &mut [u8]) -> FileSystemResult<u64>;
    fn get_total_size(&self) -> u64;
}

struct UnityBlocksReader<'a, 'b, T: ?Sized> {
    file: &'a T,
    start_pos: u64,
    blocks_info: &'b [types::UnityFSBlockInfo],
    // For quick seeking
    blocks_end_pos: Vec<UnityBlocksReaderPos>,
    cache: Mutex<Option<(usize, Vec<u8>)>>,
}
impl<'a, 'b, T: ?Sized> UnityBlocksReader<'a, 'b, T> {
    fn new(file: &'a T, start_pos: u64, blocks_info: &'b [types::UnityFSBlockInfo]) -> Self {
        // Make sure blocks_end_pos is non-empty
        assert!(!blocks_info.is_empty());
        let blocks_end_pos = blocks_info
            .iter()
            .scan(
                UnityBlocksReaderPos {
                    raw_pos: 0,
                    uncomp_pos: 0,
                },
                |s, x| {
                    s.raw_pos += x.compressed_size as u64;
                    s.uncomp_pos += x.uncompressed_size as u64;
                    Some(*s as _)
                },
            )
            .collect();
        Self {
            file,
            start_pos,
            blocks_info,
            blocks_end_pos,
            cache: Mutex::new(None),
        }
    }
}
impl<'a, 'b, T: AsRef<dyn crate::fs_provider::File + 'a> + ?Sized> ReadAt
    for UnityBlocksReader<'a, 'b, T>
{
    fn read_at(&self, offset: u64, mut buffer: &mut [u8]) -> FileSystemResult<u64> {
        let file = self.file.as_ref();
        let mut part_idx = self
            .blocks_end_pos
            .partition_point(|x| x.uncomp_pos <= offset);
        let mut block_copy_offset = None;
        let mut read_bytes = 0;
        while part_idx < self.blocks_info.len() && !buffer.is_empty() {
            let block_end_pos = &self.blocks_end_pos[part_idx];
            let block_info = &self.blocks_info[part_idx];
            let mut guard = self.cache.lock().unwrap();
            let data = match &*guard {
                Some(x) if part_idx == x.0 => &x.1,
                _ => {
                    // Read and decompress raw data
                    let raw_block_size = block_info.compressed_size as u64;
                    let raw_block_offset =
                        self.start_pos + (block_end_pos.raw_pos - raw_block_size);
                    let mut buf = vec![0; raw_block_size as _];
                    file.read_at_exact(raw_block_offset, &mut buf)?;
                    let data = decompress_data(
                        block_info.flags.compression_type(),
                        buf,
                        block_info.uncompressed_size as _,
                    )?;
                    *guard = Some((part_idx, data));
                    &guard.as_ref().unwrap().1
                }
            };
            // If this is the first read, consider read offset
            if block_copy_offset.is_none() {
                let uncomp_start_pos =
                    block_end_pos.uncomp_pos - block_info.uncompressed_size as u64;
                block_copy_offset = Some(offset - uncomp_start_pos);
            }
            // Fill buffer
            let mut data = &data[block_copy_offset.unwrap() as _..];
            let copy_len = data.read(buffer).unwrap();
            buffer = &mut buffer[copy_len..];
            read_bytes += copy_len as u64;
            // No offsets are needed for further reads
            block_copy_offset = Some(0);

            part_idx += 1;
        }
        Ok(read_bytes)
    }
    fn get_total_size(&self) -> u64 {
        self.blocks_end_pos.last().unwrap().uncomp_pos
    }
}
impl<'a, T: AsRef<dyn crate::fs_provider::File + 'a> + ?Sized> AsRef<dyn ReadAt + 'a>
    for UnityBlocksReader<'a, 'a, T>
{
    fn as_ref(&self) -> &(dyn ReadAt + 'a) {
        self
    }
}

#[derive(Clone)]
struct CursorUnityBlocksReader<T> {
    reader: T,
    position: u64,
    scope: SimpleFilePtr,
}
impl<'a, T: AsRef<dyn ReadAt + 'a>> CursorUnityBlocksReader<T> {
    fn new(reader: T) -> Self {
        Self::with_position(reader, 0)
    }
    fn with_position(reader: T, position: u64) -> Self {
        let size = reader.as_ref().get_total_size();
        Self::with_position_scope(reader, position, SimpleFilePtr { offset: 0, size })
    }
    fn with_scope(reader: T, scope: SimpleFilePtr) -> Self {
        Self::with_position_scope(reader, 0, scope)
    }
    fn with_position_scope(reader: T, position: u64, scope: SimpleFilePtr) -> Self {
        assert!(scope.offset + scope.size <= reader.as_ref().get_total_size());
        CursorUnityBlocksReader {
            reader,
            position,
            scope,
        }
    }
    fn get_position(&self) -> u64 {
        self.position
    }
    // NOTE: For nested scoping
    fn into_scoped(mut self, scope: SimpleFilePtr) -> Self {
        let offset = self.scope.offset + scope.offset;
        let size = self.scope.size.saturating_sub(scope.offset).min(scope.size);
        let scope = SimpleFilePtr { offset, size };
        self.position = 0;
        self.scope = scope;
        self
    }
    fn clone_ref(&self) -> CursorUnityBlocksReader<&T> {
        CursorUnityBlocksReader {
            reader: &self.reader,
            position: self.position,
            scope: self.scope,
        }
    }
}
impl<'a, T: AsRef<dyn ReadAt + 'a>> std::io::Read for CursorUnityBlocksReader<T> {
    fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
        if self.position >= self.scope.size {
            return Ok(0);
        }
        let scope_rest_len = (self.scope.size - self.position) as usize;
        if buf.len() > scope_rest_len {
            buf = &mut buf[..scope_rest_len];
        }
        let offset = self.position + self.scope.offset;
        let reader = self.reader.as_ref();
        let count = reader.read_at(offset, buf)?;
        self.position += count;
        Ok(count as _)
    }
}
impl<'a, T: AsRef<dyn ReadAt + 'a>> std::io::Seek for CursorUnityBlocksReader<T> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        use std::io::SeekFrom;
        match pos {
            SeekFrom::Start(pos) => {
                self.position = pos;
            }
            SeekFrom::Current(pos) => {
                self.position = self.position.saturating_add_signed(pos);
            }
            SeekFrom::End(pos) => {
                self.position = self.scope.size.saturating_add_signed(pos);
            }
        }
        Ok(self.position)
    }
}

fn read_ab_header(
    reader: &mut (impl Read + Seek),
) -> anyhow::Result<(
    types::UnityFSAssetBundle,
    types::UnityFSBlocksAndDirsInfo,
    types::UnityEngineVersion,
)> {
    let ab = types::UnityFSAssetBundle::read(reader)?;
    let engine_ver = String::from_utf8_lossy(&ab.file_engine_ver);
    let engine_ver = types::UnityEngineVersion::try_from(engine_ver.as_ref())?;

    if !ab.flags.has_dir_info() {
        anyhow::bail!("AB did not set flag has_dir_info, which is unsupported");
    }
    if ab.flags.block_and_dir_info_at_eof() {
        anyhow::bail!("AB has unsupported flag block_and_dir_info_at_eof set");
    }
    if ab.file_ver >= 7 {
        reader.align_seek(16)?;
    }

    // Read compressed blocks info
    let compression_type = ab
        .flags
        .compression_type_or_err()
        .map_err(|e| anyhow::anyhow!("got invalid compression type: {e:?}"))?;
    let blocks_data = unsafe { crate::util::read_exact_to_vec(reader, ab.compressed_size as _)? };
    let blocks_data = decompress_data(compression_type, blocks_data, ab.uncompressed_size as _)?;
    let mut blocks_data_reader = Cursor::new(&blocks_data);
    let blocks_and_dirs_info = types::UnityFSBlocksAndDirsInfo::read(&mut blocks_data_reader)?;

    if ab.flags.has_padding_before_blocks() {
        reader.align_seek(16)?;
    }

    Ok((ab, blocks_and_dirs_info, engine_ver))
}

impl<'a> UnityArchive<'a> {
    pub(super) fn new(
        open_ctx: ArchiveHandlerOpenContext<'a>,
        _non_unicode_compat: &ArchiveNonUnicodeCompatConfig,
    ) -> FileSystemResult<Self> {
        use crate::fs_provider::File;
        use std::collections::btree_map::Entry::*;

        // NOTE: Empty asset_base_name uses the primary file for reading
        fn add_file_to_root_dir(
            root_dir: &mut UnityFolderEntry,
            root_index: u64,
            // container: &types::UnityAlignedString,
            container: &Cow<'_, str>,
            dir_finder: impl FnOnce(&str) -> SimpleFilePtr,
            obj_info: &types::UnityFSSerializedObjectInfo,
            obj: types::UnityFSSerializedObject,
            asset_base_name: CaselessString,
        ) -> anyhow::Result<()> {
            use types::UnityFSSerializedObject::*;

            // let container = String::from_utf8_lossy(&container.0);
            let path = SegPath::new_truncate(&container, PathDelimiter::Slash);
            let mut cur_dir_children = &mut root_dir.children;
            let mut iter = path.iter().peekable();
            let mut filename = "";

            // NOTE: Used for "unique" index generation
            let mut counter: u64 = 0;

            while let Some(path) = iter.next() {
                if iter.peek().is_none() {
                    filename = path;
                    break;
                }

                let key = CaselessString::new(path.to_owned());
                cur_dir_children = match cur_dir_children.entry(key) {
                    Occupied(e) => match e.into_mut() {
                        UnityEntry::File(_) => anyhow::bail!("file name collides with folder"),
                        UnityEntry::Folder(e) => &mut e.children,
                    },
                    Vacant(e) => {
                        match e.insert(UnityEntry::Folder(UnityFolderEntry {
                            children: BTreeMap::new(),
                            index: calculate_hash(&(root_index, counter)),
                        })) {
                            UnityEntry::Folder(e) => {
                                counter += 1;
                                &mut e.children
                            }
                            _ => unreachable!(),
                        }
                    }
                };
            }
            if filename.is_empty() {
                anyhow::bail!("file name is empty");
            }
            let key = CaselessString::new(filename.to_owned());
            let extract_name_from_path_fn = |s| match String::from_utf8_lossy(s) {
                Cow::Borrowed(s) => {
                    Cow::Borrowed(s.rsplit_once('/').map(|x| x.1).unwrap_or_default())
                }
                Cow::Owned(s) => Cow::Owned(
                    s.rsplit_once('/')
                        .map(|x| x.1.to_owned())
                        .unwrap_or_default(),
                ),
            };
            let file_ptr = match &obj {
                Texture2D(info) => UnityFilePtr {
                    base_name: asset_base_name,
                    inner_offset: Some(dir_finder(
                        extract_name_from_path_fn(&info.stream_data.path.0).as_ref(),
                    )),
                    offset: info.stream_data.offset,
                    size: info.stream_data.size as _,
                },
                _ => anyhow::bail!("unimplemented object type: {obj:?}"),
            };
            match cur_dir_children.entry(key) {
                Occupied(_) => anyhow::bail!("file name collides with file"),
                Vacant(e) => {
                    e.insert(UnityEntry::File(UnityFileEntry {
                        file_ptr,
                        index: obj_info.path_id,
                        obj,
                    }));
                }
            }

            Ok(())
        }

        fn process_assets_file(
            file: &dyn File,
            root_dir: &mut UnityFolderEntry,
        ) -> anyhow::Result<()> {
            // TODO...
            todo!()
        }

        fn process_ab_file(
            file: &dyn File,
            filename: &str,
            root_dir: &mut UnityFolderEntry,
            root_index: u64,
        ) -> anyhow::Result<()> {
            use num_traits::FromPrimitive;
            use types::UnityObjectClassIDType;

            let mut cursor_file = CursorFile::new(file);
            let file_type = guess_file_type(&mut cursor_file)?;
            // TODO: Also handle AssetsFile
            if file_type != UnityFileType::BundleFile {
                anyhow::bail!("unimplemented: file_type != UnityFileType::BundleFile");
            }
            // TODO: Handle orphan objects (i.e. has no containers)

            // Read AB headers (header + blocks info)
            let (ab, blocks_and_dirs_info, engine_ver) = read_ab_header(&mut cursor_file)?;

            // Read assets list
            let blocks_reader = UnityBlocksReader::new(
                file,
                cursor_file.get_position(),
                &blocks_and_dirs_info.blocks_info,
            );
            // NOTE: The map is only for local lookup, and will not be persisted
            let mut dirs_map = HashMap::new();
            for i in blocks_and_dirs_info.dirs_info.iter() {
                let v = SimpleFilePtr {
                    offset: i.offset,
                    size: i.size,
                };
                dirs_map.insert(String::from_utf8_lossy(&i.path), v);
            }

            let mut assets_file = None;
            let mut assets_file_ptr = SimpleFilePtr { offset: 0, size: 0 };
            for (k, &v) in dirs_map.iter() {
                let mut file_reader = CursorUnityBlocksReader::with_scope(&blocks_reader, v);
                let file_type = guess_file_type(&mut file_reader)?;
                if let UnityFileType::AssetsFile = file_type {
                    assets_file = Some(file_reader);
                    assets_file_ptr = v;
                    break;
                }
            }
            let (serialized_info, file_reader) = match assets_file {
                Some(mut reader) => (types::UnityFSSerializedFileInfo::read(&mut reader)?, reader),
                _ => anyhow::bail!("assets file not found in AB"),
            };
            let objs_data_reader = file_reader.into_scoped(SimpleFilePtr {
                offset: serialized_info.data_offset,
                size: serialized_info.file_size,
            });
            let endian = if serialized_info.endianness == 0 {
                binrw::Endian::Little
            } else {
                binrw::Endian::Big
            };
            let version = serialized_info.version;
            let target_platform = serialized_info.target_platform;

            // Find AssetBundle inside AssetsFile
            let mut ab_info = None;
            let mut path_id_name_map = HashMap::new();
            for obj_info in serialized_info.objects.iter() {
                let cid = <UnityObjectClassIDType as FromPrimitive>::from_u32(obj_info.class_id);
                if let Some(UnityObjectClassIDType::AssetBundle) = cid {
                    let scope = SimpleFilePtr {
                        offset: obj_info.byte_start,
                        size: obj_info.byte_size as _,
                    };
                    let mut obj_reader = objs_data_reader.clone().into_scoped(scope);
                    ab_info = Some(types::UnityFSSerializedAssetBundleObjectInfo::read_options(
                        &mut obj_reader,
                        endian,
                        binrw::args! { version, target_platform },
                    )?);
                    for (cont, infos) in &ab_info.as_ref().unwrap().containers {
                        let cont = String::from_utf8_lossy(&cont.0);
                        path_id_name_map
                            .extend(infos.iter().map(|x| (x.asset_pptr.path_id, cont.clone())));
                    }
                    break;
                }
            }
            // TODO: Remove this
            if path_id_name_map.is_empty() {
                anyhow::bail!("AB does not exist, and no original path info can be recovered");
            }

            // NOTE: We ignore all read errors for objects
            for obj_info in serialized_info.objects.iter() {
                let obj_container = match path_id_name_map.get(&obj_info.path_id) {
                    Some(x) => x,
                    None => continue,
                };
                let scope = SimpleFilePtr {
                    offset: obj_info.byte_start,
                    size: obj_info.byte_size as _,
                };
                let mut obj_reader = objs_data_reader.clone().into_scoped(scope);
                let obj = match types::UnityFSSerializedObject::read_options(
                    &mut obj_reader,
                    endian,
                    binrw::args! { class_id: obj_info.class_id, version, target_platform, engine_ver },
                ) {
                    Ok(x) => x,
                    Err(e) => {
                        log::debug!("skipping reading object {}: {e}", obj_info.path_id);
                        continue;
                    }
                };
                add_file_to_root_dir(
                    root_dir,
                    root_index,
                    obj_container,
                    |k| *dirs_map.get(k).unwrap_or(&assets_file_ptr),
                    obj_info,
                    obj,
                    filename.into(),
                )?;
            }

            Ok(())
        }

        let root_file_index = open_ctx.get_file().get_stat()?.index;

        if open_ctx.get_is_dir() {
            // Handle directory
            Err(FileSystemError::NotImplemented)
        } else {
            // Handle file
            let file = open_ctx.get_file();
            let filename = open_ctx.get_file_name();

            let mut root_dir = UnityFolderEntry {
                children: BTreeMap::new(),
                index: root_file_index,
            };

            process_ab_file(file, filename, &mut root_dir, root_file_index)?;

            Ok(Self { open_ctx, root_dir })
        }
    }

    // TODO: Remove this
    // fn read_file(&self, file_ptr: UnityFilePtr) -> FileSystemResult<Vec<u8>> {
    //     return Err(FileSystemError::NotImplemented);
    // }
}

impl UnityArchive<'_> {
    fn resolve_path<'a, 's>(
        &'a self,
        path: SegPath<'s>,
    ) -> FileSystemResult<(Option<&'a UnityFolderEntry>, &'s str)> {
        let mut parent: Option<&UnityFolderEntry> = None;
        let mut cur_dir = &self.root_dir;
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
            let next_dir = if let Some(UnityEntry::Folder(folder)) =
                cur_dir.children.get(CaselessStr::new(path))
            {
                folder
            } else {
                return Err(FileSystemError::ObjectPathNotFound);
            };
            cur_dir = next_dir;
        }
        if non_empty {
            parent = Some(cur_dir);
        }
        Ok((parent, filename))
    }
}

impl super::ArchiveHandler for UnityArchive<'_> {
    fn open_file(
        &self,
        filename: crate::fs_provider::SegPath,
    ) -> FileSystemResult<super::ArchiveHandlerOpenFileInfo<'_>> {
        let (parent, filename) = self.resolve_path(filename)?;

        let entry = if let Some(parent) = parent {
            parent
                .children
                .get(CaselessStr::new(filename))
                .ok_or(FileSystemError::ObjectNameNotFound)?
                .as_borrowed()
        } else {
            BorrowedUnityEntry::Folder(&self.root_dir)
        };

        Ok(super::ArchiveHandlerOpenFileInfo {
            context: match entry {
                BorrowedUnityEntry::Folder(e) => Box::new(UnityFolderFile { entry: e }),
                BorrowedUnityEntry::File(e) => {
                    let filename = SegPath::new_truncate(
                        e.file_ptr.base_name.as_str(),
                        PathDelimiter::BackSlash,
                    );
                    let open_result = self.open_ctx.open_file(filename)?;
                    if open_result.is_dir {
                        return Err(FileSystemError::FileCorruptError);
                    }
                    let fs_file = open_result.context;
                    let mut cursor_file = CursorFile::new(&*fs_file);
                    let mut cursor_file = binrw::io::BufReader::new(cursor_file);
                    let (ab, blocks_and_dirs_info, engine_ver) = read_ab_header(&mut cursor_file)?;
                    // TODO: SAFETY statement
                    let reader = unsafe {
                        std::mem::transmute(UnityBlocksReader::new(
                            fs_file.as_ref(),
                            cursor_file
                                .stream_position()
                                .map_err(|_| FileSystemError::FileCorruptError)?,
                            &blocks_and_dirs_info.blocks_info,
                        ))
                    };
                    let reader = CursorUnityBlocksReader::with_scope(
                        reader,
                        e.file_ptr.inner_offset.unwrap(),
                    );
                    let reader = reader.into_scoped(SimpleFilePtr {
                        offset: e.file_ptr.offset,
                        size: e.file_ptr.size,
                    });
                    Box::new(UnityFileFile {
                        root_archive: self,
                        entry: e,
                        reader,
                        blocks_info: blocks_and_dirs_info.blocks_info,
                        cache: RwLock::new(None),
                    })
                }
            },
            is_dir: entry.is_dir(),
        })
    }
}

struct UnityFolderFile<'a> {
    entry: &'a UnityFolderEntry,
}

impl super::ArchiveFile for UnityFolderFile<'_> {
    fn read_at(&self, _offset: u64, _buffer: &mut [u8]) -> FileSystemResult<u64> {
        Err(FileSystemError::FileIsADirectory)
    }
    fn get_stat(&self) -> FileSystemResult<FileStatInfo> {
        Ok(self.entry.get_file_stat_info())
    }
    fn find_files_with_pattern(
        &self,
        pattern: &dyn crate::fs_provider::FilePattern,
        filler: &mut dyn crate::fs_provider::FindFilesDataFiller,
    ) -> FileSystemResult<()> {
        for (name, child) in self
            .entry
            .children
            .iter()
            .filter(|(name, _)| pattern.check_name(name.as_str()))
        {
            let mut stat = child.get_file_stat_info();
            if let UnityEntry::File(e) = child {
                stat.size = get_obj_file_size(&e.obj);
            }
            if filler.fill_data(name.as_str(), &stat).is_err() {
                log::warn!("Failed to fill object data");
            }
        }
        Ok(())
    }
}

struct UnityFileFile<'a> {
    root_archive: &'a UnityArchive<'a>,
    entry: &'a UnityFileEntry,
    // TODO: Correctly annonate lifetime
    // NOTE: reader must be dropped before blocks_info
    reader: CursorUnityBlocksReader<UnityBlocksReader<'a, 'static, dyn crate::fs_provider::File>>,
    blocks_info: Vec<types::UnityFSBlockInfo>,
    cache: RwLock<Option<Vec<u8>>>,
}

const BMP_HDR_SIZE: u64 = 14;
const BITMAPV4HEADER_SIZE: u64 = 108;

fn get_obj_file_size(obj: &types::UnityFSSerializedObject) -> u64 {
    use types::UnityFSSerializedObject::*;
    match obj {
        Texture2D(obj) => {
            // TODO: Generate uncompressed png instead, as not all apps
            //       can handle alpha BMPs very well
            let pixels_size = 4 * obj.width as u64 * obj.height as u64;
            BMP_HDR_SIZE + BITMAPV4HEADER_SIZE + pixels_size
        }
        _ => 0,
    }
}

impl super::ArchiveFile for UnityFileFile<'_> {
    fn read_at(&self, offset: u64, buffer: &mut [u8]) -> FileSystemResult<u64> {
        use binrw::BinWrite;
        use num_traits::FromPrimitive;
        use types::UnityFSSerializedObject::*;
        use types::UnityTextureFormat;

        let cache = loop {
            let guard = self.cache.read().unwrap();
            if guard.is_some() {
                break guard;
            }
            drop(guard);
            match *self.cache.write().unwrap() {
                ref mut data @ None => {
                    *data = Some(match &self.entry.obj {
                        Texture2D(obj) => || -> anyhow::Result<_> {
                            let mut reader = self.reader.clone_ref();
                            let texture_format =
                                <UnityTextureFormat as FromPrimitive>::from_u32(obj.texture_format)
                                    .context("invalid texture format")?;
                            if texture_format != UnityTextureFormat::RGBA32 {
                                anyhow::bail!("unsupported texture format {texture_format}");
                            }
                            let pixels_size = 4 * obj.width as u64 * obj.height as u64;
                            let mut data = Vec::new();
                            let mut data_noseek = binrw::io::NoSeek::new(&mut data);
                            // BMP header
                            b"BM".write_le(&mut data_noseek)?;
                            (get_obj_file_size(&self.entry.obj) as u32)
                                .write_le(&mut data_noseek)?;
                            0u16.write_le(&mut data_noseek)?;
                            0u16.write_le(&mut data_noseek)?;
                            ((BMP_HDR_SIZE + BITMAPV4HEADER_SIZE) as u32)
                                .write_le(&mut data_noseek)?;
                            // DIB header
                            (BITMAPV4HEADER_SIZE as u32).write_le(&mut data_noseek)?;
                            (obj.width as u32).write_le(&mut data_noseek)?;
                            (obj.height as u32).write_le(&mut data_noseek)?;
                            1u16.write_le(&mut data_noseek)?;
                            32u16.write_le(&mut data_noseek)?;
                            3u32.write_le(&mut data_noseek)?;
                            (pixels_size as u32).write_le(&mut data_noseek)?;
                            0u32.write_le(&mut data_noseek)?;
                            0u32.write_le(&mut data_noseek)?;
                            0u32.write_le(&mut data_noseek)?;
                            0u32.write_le(&mut data_noseek)?;
                            // ARGB masks
                            //  R
                            0xff0000u32.write_le(&mut data_noseek)?;
                            //  G
                            0xff00u32.write_le(&mut data_noseek)?;
                            //  B
                            0xffu32.write_le(&mut data_noseek)?;
                            //  A
                            0xff000000u32.write_le(&mut data_noseek)?;
                            // Colorspace
                            0x73524742.write_le(&mut data_noseek)?;
                            [0u32; 12].write_le(&mut data_noseek)?;
                            let headers_len = data.len();
                            data.reserve(pixels_size as _);
                            unsafe {
                                reader.read_exact(core::slice::from_raw_parts_mut(
                                    data.as_mut_ptr().add(headers_len),
                                    pixels_size as _,
                                ))?;
                                data.set_len(headers_len + pixels_size as usize);
                            }
                            // Swap colors to make BGRA format
                            for i in data[headers_len..].chunks_exact_mut(4) {
                                (i[0], i[2]) = (i[2], i[0]);
                            }
                            Ok(data)
                        }()?,
                        _ => return Err(FileSystemError::NotImplemented),
                    })
                }
                _ => (),
            }
            break self.cache.read().unwrap();
        };
        // SAFETY: cache is already initialized
        let data = unsafe { cache.as_ref().unwrap_unchecked() };

        if offset as usize >= data.len() {
            Ok(0)
        } else {
            let mut src = &data[offset as _..];
            src.read(buffer)
                .map(|x| x as _)
                .map_err(|e| FileSystemError::Other(e.into()))
        }
    }
    fn get_stat(&self) -> crate::fs_provider::FileSystemResult<FileStatInfo> {
        let mut stat = self.entry.get_file_stat_info();
        stat.size = get_obj_file_size(&self.entry.obj);
        Ok(stat)
    }
    fn find_files_with_pattern(
        &self,
        _pattern: &dyn crate::fs_provider::FilePattern,
        _filler: &mut dyn crate::fs_provider::FindFilesDataFiller,
    ) -> FileSystemResult<()> {
        Err(FileSystemError::NotADirectory)
    }
}
