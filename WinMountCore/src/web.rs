use std::{
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc, Mutex},
    time::SystemTime,
};

use axum::{
    extract::{ws::WebSocket, FromRef, State, WebSocketUpgrade},
    response::Response,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::fs_provider::FileSystemError;

use crate::util::parse_u32;
const SERVER_MAJOR: u32 = parse_u32(env!("CARGO_PKG_VERSION_MAJOR"));
const SERVER_MINOR: u32 = parse_u32(env!("CARGO_PKG_VERSION_MINOR"));
const SERVER_PATCH: u32 = parse_u32(env!("CARGO_PKG_VERSION_PATCH"));

// NOTE: Request is followed by a Response with the same syn number
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub(super) enum WsMessage {
    Failure {
        code: i32,
        msg: String,
    },
    Request {
        syn: u64,
        method: String,
        #[serde(default)]
        params: serde_json::Value,
    },
    Response {
        syn: u64,
        code: i32,
        msg: String,
        data: serde_json::Value,
    },
    // NOTE: To start receiving subscriptions, one must send a request
    //       responsible for PubSub registration
    Subscription {
        id: u64,
        name: String,
        payload: serde_json::Value,
    },
}
impl WsMessage {
    fn new_resp_ok(syn: u64, data: serde_json::Value) -> Self {
        Self::Response {
            syn,
            code: 0,
            msg: String::new(),
            data,
        }
    }
    fn new_resp_err(syn: u64, code: i32, msg: String) -> Self {
        Self::Response {
            syn,
            code,
            msg,
            data: serde_json::Value::Null,
        }
    }
}

// Hardcoded binary message types (not designed for extensibility)
#[derive(Serialize, Deserialize, Debug)]
pub(super) enum WsBinMessage<'a> {
    // The response for general failure
    Failure {
        code: i32,
        msg: String,
    },
    ReadFileReq {
        id: u64,
        file_id: u64,
        offset: u64,
        size: u64,
    },
    ReadFileResp {
        id: u64,
        file_id: u64,
        status: i32,
        payload: &'a [u8],
    },
    WriteFileReq {
        id: u64,
        file_id: u64,
        offset: u64,
        payload: &'a [u8],
    },
    WriteFileResp {
        id: u64,
        file_id: u64,
        status: i32,
        written_bytes: u64,
    },
}

// WARN: A self-referential sealed struct which uses **unsafe**
struct FileSystemWithChildren {
    handler: Arc<dyn crate::fs_provider::FileSystemHandler>,
    // NOTE: Not to be confused with file.stat.index;
    //       the id (key) is only for the current session
    files: HashMap<u64, *mut dyn crate::fs_provider::File>,
    id_counter: AtomicU64,
}
unsafe impl Send for FileSystemWithChildren {}
unsafe impl Sync for FileSystemWithChildren {}
impl Drop for FileSystemWithChildren {
    fn drop(&mut self) {
        self.clear_open_files();
    }
}

#[derive(Serialize)]
struct FileSystemWithChildrenOpenFileData {
    id: u64,
    is_dir: bool,
    new_file_created: bool,
}
#[derive(Serialize)]
struct FileSystemWithChildrenFileStatData {
    size: u64,
    is_dir: bool,
    creation_time: SystemTime,
    last_access_time: SystemTime,
    last_write_time: SystemTime,
}
impl FileSystemWithChildren {
    fn new(handler: Arc<dyn crate::fs_provider::FileSystemHandler>) -> Self {
        Self {
            handler,
            files: HashMap::new(),
            id_counter: AtomicU64::new(0),
        }
    }
    unsafe fn drop_file_ptr(file: *mut dyn crate::fs_provider::File) {
        let _file = Box::from_raw(file);
    }
    fn clear_open_files(&mut self) {
        self.files.retain(|_, v| {
            unsafe {
                Self::drop_file_ptr(*v);
            }
            false
        });
    }
    fn has_open_files(&self) -> bool {
        !self.files.is_empty()
    }
    fn open_file(
        &mut self,
        path: &str,
        can_write: bool,
    ) -> Result<FileSystemWithChildrenOpenFileData, FileSystemError> {
        use crate::fs_provider::*;
        use std::sync::atomic::Ordering;
        let path = SegPath::new(path, PathDelimiter::Slash);
        let result = self.handler.create_file(
            path,
            if can_write {
                FileDesiredAccess::ReadWrite
            } else {
                FileDesiredAccess::Read
            },
            FileAttributes::empty(),
            FileShareAccess::all(),
            if can_write {
                FileCreateDisposition::OpenAlways
            } else {
                FileCreateDisposition::OpenExisting
            },
            FileCreateOptions::empty(),
        )?;
        let id = self.id_counter.fetch_add(1, Ordering::Relaxed);
        // HACK: Extend lifetime to 'static
        let file_ptr = unsafe { std::mem::transmute(Box::into_raw(result.context)) };
        if let Some(file) = self.files.insert(id, file_ptr) {
            // Too many files open, drop old ones
            unsafe {
                Self::drop_file_ptr(file);
            }
        }
        Ok(FileSystemWithChildrenOpenFileData {
            id,
            is_dir: result.is_dir,
            new_file_created: result.new_file_created,
        })
    }
    fn close_file(&mut self, id: u64) -> Result<(), ()> {
        use std::collections::hash_map::Entry::*;
        match self.files.entry(id) {
            Occupied(e) => {
                unsafe {
                    Self::drop_file_ptr(e.remove());
                }
                Ok(())
            }
            Vacant(_) => Err(()),
        }
    }
    fn stat_file(&self, id: u64) -> Result<FileSystemWithChildrenFileStatData, FileSystemError> {
        let file = unsafe {
            &**self
                .files
                .get(&id)
                .ok_or(FileSystemError::InvalidParameter)?
        };
        let stat = file.get_stat()?;
        Ok(FileSystemWithChildrenFileStatData {
            size: stat.size,
            is_dir: stat.is_dir,
            creation_time: stat.creation_time,
            last_access_time: stat.last_access_time,
            last_write_time: stat.last_write_time,
        })
    }
    fn read_file_at(
        &self,
        id: u64,
        offset: u64,
        buffer: &mut [u8],
    ) -> Result<u64, FileSystemError> {
        let file = unsafe {
            &**self
                .files
                .get(&id)
                .ok_or(FileSystemError::InvalidParameter)?
        };
        file.read_at(offset, buffer)
    }
    fn write_file_at(
        &self,
        id: u64,
        offset: Option<u64>,
        buffer: &[u8],
        constrain_size: bool,
    ) -> Result<u64, FileSystemError> {
        let file = unsafe {
            &**self
                .files
                .get(&id)
                .ok_or(FileSystemError::InvalidParameter)?
        };
        file.write_at(offset, buffer, constrain_size)
    }
    fn list_files(
        &self,
        id: u64,
    ) -> Result<Vec<(String, FileSystemWithChildrenFileStatData)>, FileSystemError> {
        struct FilesDataFiller<'a> {
            vec: &'a mut Vec<(String, FileSystemWithChildrenFileStatData)>,
        }
        impl crate::fs_provider::FindFilesDataFiller for FilesDataFiller<'_> {
            fn fill_data(
                &mut self,
                name: &str,
                stat: &crate::fs_provider::FileStatInfo,
            ) -> Result<(), ()> {
                self.vec.try_reserve(1).map_err(|_| ())?;
                self.vec.push((
                    name.to_owned(),
                    FileSystemWithChildrenFileStatData {
                        size: stat.size,
                        is_dir: stat.is_dir,
                        creation_time: stat.creation_time,
                        last_access_time: stat.last_access_time,
                        last_write_time: stat.last_write_time,
                    },
                ));
                Ok(())
            }
        }
        let file = unsafe {
            &**self
                .files
                .get(&id)
                .ok_or(FileSystemError::InvalidParameter)?
        };
        let mut files_list = Vec::new();
        file.find_files_with_pattern(
            &crate::fs_provider::AcceptAllFilePattern::new(),
            &mut FilesDataFiller {
                vec: &mut files_list,
            },
        )?;
        Ok(files_list)
    }
    // fn list_files_with_pattern(&self, id: u64, pattern: &str) -> Result<Vec<(String, FileSystemWithChildrenFileStatData)>, FileSystemError> {
    //     let file = unsafe {
    //         &**self
    //             .files
    //             .get(&id)
    //             .ok_or(FileSystemError::InvalidParameter)?
    //     };
    //     file.find_files_with_pattern(pattern, filler)
    // }
}

struct WsFileSystemContext<'a> {
    app_ctx: &'a Arc<Mutex<crate::AppContext>>,
    fs: HashMap<Uuid, FileSystemWithChildren>,
}
impl<'a> WsFileSystemContext<'a> {
    fn new(app_ctx: &'a Arc<Mutex<crate::AppContext>>) -> Self {
        Self {
            app_ctx,
            fs: HashMap::new(),
        }
    }
    fn open_file(
        &mut self,
        fs_id: Uuid,
        path: &str,
        can_write: bool,
    ) -> Result<FileSystemWithChildrenOpenFileData, FileSystemError> {
        use std::collections::hash_map::Entry::*;
        Ok(match self.fs.entry(fs_id) {
            Occupied(mut e) => {
                let fs = e.get_mut();
                fs.open_file(path, can_write)?
            }
            Vacant(e) => {
                let fs = Arc::clone(
                    self.app_ctx
                        .lock()
                        .unwrap()
                        .filesystems
                        .get(&fs_id)
                        .ok_or(FileSystemError::InvalidParameter)?
                        .handler
                        .as_ref()
                        .ok_or(FileSystemError::InvalidParameter)?,
                );
                let mut fs = FileSystemWithChildren::new(fs);
                let open_data = fs.open_file(path, can_write)?;
                e.insert(fs);
                open_data
            }
        })
    }
    fn close_file(&mut self, fs_id: Uuid, id: u64) -> Result<(), ()> {
        use std::collections::hash_map::Entry::*;
        match self.fs.entry(fs_id) {
            Occupied(mut e) => {
                let fs = e.get_mut();
                fs.close_file(id)?;
                if !fs.has_open_files() {
                    e.remove();
                }
                Ok(())
            }
            Vacant(_) => Err(()),
        }
    }
    fn stat_file(
        &self,
        fs_id: Uuid,
        id: u64,
    ) -> Result<FileSystemWithChildrenFileStatData, FileSystemError> {
        let fs = self
            .fs
            .get(&fs_id)
            .ok_or(FileSystemError::InvalidParameter)?;
        fs.stat_file(id)
    }
    fn read_file_at(
        &self,
        fs_id: Uuid,
        id: u64,
        offset: u64,
        buffer: &mut [u8],
    ) -> Result<u64, FileSystemError> {
        let fs = self
            .fs
            .get(&fs_id)
            .ok_or(FileSystemError::InvalidParameter)?;
        fs.read_file_at(id, offset, buffer)
    }
    fn write_file_at(
        &self,
        fs_id: Uuid,
        id: u64,
        offset: Option<u64>,
        buffer: &[u8],
        constrain_size: bool,
    ) -> Result<u64, FileSystemError> {
        let fs = self
            .fs
            .get(&fs_id)
            .ok_or(FileSystemError::InvalidParameter)?;
        fs.write_file_at(id, offset, buffer, constrain_size)
    }
    fn list_files(
        &self,
        fs_id: Uuid,
        id: u64,
    ) -> Result<Vec<(String, FileSystemWithChildrenFileStatData)>, FileSystemError> {
        let fs = self
            .fs
            .get(&fs_id)
            .ok_or(FileSystemError::InvalidParameter)?;
        fs.list_files(id)
    }
}

async fn handle_websocket_bin_request<'b>(
    socket: &mut WebSocket,
    app_ctx: &Arc<Mutex<crate::AppContext>>,
    fs_ctx: &mut WsFileSystemContext<'_>,
    b: &'b mut Vec<u8>,
) -> anyhow::Result<WsBinMessage<'b>> {
    // TODO...
    Ok(WsBinMessage::Failure {
        code: -1,
        msg: "unknown error".to_owned(),
    })
}

async fn handle_websocket_send_bin_msg(
    socket: &mut WebSocket,
    msg: WsBinMessage<'_>,
) -> anyhow::Result<()> {
    let msg = axum::extract::ws::Message::Binary(bincode::serialize(&msg)?);
    socket.send(msg).await?;
    Ok(())
}

async fn handle_websocket_send_msg(socket: &mut WebSocket, msg: WsMessage) -> anyhow::Result<()> {
    let msg = axum::extract::ws::Message::Text(serde_json::to_string(&msg)?);
    socket.send(msg).await?;
    Ok(())
}

async fn handle_websocket_request(
    socket: &mut WebSocket,
    app_ctx: &Arc<Mutex<crate::AppContext>>,
    fs_ctx: &mut WsFileSystemContext<'_>,
    syn: u64,
    method: String,
    params: serde_json::Value,
) -> anyhow::Result<WsMessage> {
    let resp_data = match method.as_str() {
        "create-fs" => {
            #[derive(Deserialize)]
            struct Params {
                name: String,
                kind_id: Uuid,
                #[serde(default)]
                config: serde_json::Value,
            }
            let params: Params = serde_json::from_value(params)?;
            let mut app_ctx = app_ctx.lock().unwrap();
            let fs_id = app_ctx.create_fs(params.name, params.kind_id, params.config)?;
            serde_json::json!({ "fs_id": fs_id })
        }
        "remove-fs" => {
            #[derive(Deserialize)]
            struct Params {
                id: Uuid,
            }
            let params: Params = serde_json::from_value(params)?;
            let mut app_ctx = app_ctx.lock().unwrap();
            app_ctx.remove_fs(params.id)?;
            serde_json::json!({})
        }
        "start-fs" => {
            #[derive(Deserialize)]
            struct Params {
                id: Uuid,
            }
            let params: Params = serde_json::from_value(params)?;
            let mut app_ctx = app_ctx.lock().unwrap();
            let new_started = app_ctx.start_fs(params.id)?;
            serde_json::json!({ "new_started": new_started })
        }
        "stop-fs" => {
            #[derive(Deserialize)]
            struct Params {
                id: Uuid,
            }
            let params: Params = serde_json::from_value(params)?;
            let mut app_ctx = app_ctx.lock().unwrap();
            let new_stopped = app_ctx.stop_fs(params.id)?;
            serde_json::json!({ "new_stopped": new_stopped })
        }
        "create-fsrv" => {
            #[derive(Deserialize)]
            struct Params {
                name: String,
                kind_id: Uuid,
                in_fs_id: Uuid,
                #[serde(default)]
                config: serde_json::Value,
            }
            let params: Params = serde_json::from_value(params)?;
            let mut app_ctx = app_ctx.lock().unwrap();
            let fsrv_id =
                app_ctx.create_fsrv(params.name, params.kind_id, params.in_fs_id, params.config)?;
            serde_json::json!({ "fsrv_id": fsrv_id })
        }
        "remove-fsrv" => {
            #[derive(Deserialize)]
            struct Params {
                id: Uuid,
            }
            let params: Params = serde_json::from_value(params)?;
            let mut app_ctx = app_ctx.lock().unwrap();
            app_ctx.remove_fsrv(params.id)?;
            serde_json::json!({})
        }
        "start-fsrv" => {
            #[derive(Deserialize)]
            struct Params {
                id: Uuid,
            }
            let params: Params = serde_json::from_value(params)?;
            let mut app_ctx = app_ctx.lock().unwrap();
            let new_started = app_ctx.start_fsrv(params.id)?;
            serde_json::json!({ "new_started": new_started })
        }
        "stop-fsrv" => {
            #[derive(Deserialize)]
            struct Params {
                id: Uuid,
            }
            let params: Params = serde_json::from_value(params)?;
            let mut app_ctx = app_ctx.lock().unwrap();
            let new_stopped = app_ctx.stop_fsrv(params.id)?;
            serde_json::json!({ "new_stopped": new_stopped })
        }
        "list-fs" => {
            let mut app_ctx = app_ctx.lock().unwrap();
            let fs_list = app_ctx.list_fs()?;
            serde_json::json!({ "fs_list": fs_list })
        }
        "list-fsp" => {
            let mut app_ctx = app_ctx.lock().unwrap();
            let fsp_list = app_ctx.list_fsp()?;
            serde_json::json!({ "fsp_list": fsp_list })
        }
        "list-fsrv" => {
            let mut app_ctx = app_ctx.lock().unwrap();
            let fsrv_list = app_ctx.list_fsrv()?;
            serde_json::json!({ "fsrv_list": fsrv_list })
        }
        "list-fsrvp" => {
            let mut app_ctx = app_ctx.lock().unwrap();
            let fsrvp_list = app_ctx.list_fsrvp()?;
            serde_json::json!({ "fsrvp_list": fsrvp_list })
        }
        "get-fs-info" => {
            #[derive(Deserialize)]
            struct Params {
                id: Uuid,
            }
            let params: Params = serde_json::from_value(params)?;
            let mut app_ctx = app_ctx.lock().unwrap();
            let info = app_ctx.get_fs_info(params.id)?;
            serde_json::json!(info)
        }
        "get-fsrv-info" => {
            #[derive(Deserialize)]
            struct Params {
                id: Uuid,
            }
            let params: Params = serde_json::from_value(params)?;
            let mut app_ctx = app_ctx.lock().unwrap();
            let info = app_ctx.get_fsrv_info(params.id)?;
            serde_json::json!(info)
        }
        "update-fs-info" => {
            #[derive(Deserialize)]
            struct Params {
                id: Uuid,
                name: Option<String>,
                config: Option<serde_json::Value>,
            }
            let params: Params = serde_json::from_value(params)?;
            let mut app_ctx = app_ctx.lock().unwrap();
            app_ctx.update_fs_info(params.id, params.name, params.config)?;
            serde_json::json!({})
        }
        "update-fsrv-info" => {
            #[derive(Deserialize)]
            struct Params {
                id: Uuid,
                name: Option<String>,
                config: Option<serde_json::Value>,
            }
            let params: Params = serde_json::from_value(params)?;
            let mut app_ctx = app_ctx.lock().unwrap();
            app_ctx.update_fsrv_info(params.id, params.name, params.config)?;
            serde_json::json!({})
        }
        // ----- START Filesystem operations ----- (context is bound to a specific session)
        "open-fs-file" => {
            #[derive(Deserialize)]
            struct Params {
                fs_id: Uuid,
                path: String,
                can_write: bool,
            }
            let params: Params = serde_json::from_value(params)?;
            let open_data = fs_ctx.open_file(params.fs_id, &params.path, params.can_write)?;
            serde_json::json!(open_data)
        }
        "close-fs-file" => {
            #[derive(Deserialize)]
            struct Params {
                fs_id: Uuid,
                id: u64,
            }
            let params: Params = serde_json::from_value(params)?;
            fs_ctx
                .close_file(params.fs_id, params.id)
                .map_err(|_| FileSystemError::InvalidParameter)?;
            serde_json::json!({})
        }
        "ls-fs-content" => {
            #[derive(Deserialize)]
            struct Params {
                fs_id: Uuid,
                id: u64,
            }
            let params: Params = serde_json::from_value(params)?;
            let files_list = fs_ctx.list_files(params.fs_id, params.id)?;
            serde_json::json!(files_list)
        }
        // NOTE: Replaced by their binary variants
        // "read-fs-file" => (),
        // "write-fs-file" => (),
        "stat-fs-file" => {
            #[derive(Deserialize)]
            struct Params {
                fs_id: Uuid,
                id: u64,
            }
            let params: Params = serde_json::from_value(params)?;
            let stat_info = fs_ctx.list_files(params.fs_id, params.id)?;
            serde_json::json!(stat_info)
        }
        // ----- END Filesystem operations -----
        _ => {
            log::warn!(
                "Received unknown request: method = {}, params = {}",
                method,
                params
            );
            anyhow::bail!("unsupported request");
        }
    };
    Ok(WsMessage::new_resp_ok(syn, resp_data))
}

async fn handle_websocket(socket: &mut WebSocket, app_ctx: Arc<Mutex<crate::AppContext>>) {
    use axum::extract::ws::Message;

    // Check version first
    if let Some(Ok(Message::Text(s))) = socket.recv().await {
        let mut accept_connection = true;
        let (mut client_major, mut client_minor, mut client_patch): (u32, u32, u32) = (0, 0, 0);
        if let Err(e) = scanf::sscanf!(
            &s,
            "WinMount connect v{}.{}.{}",
            client_major,
            client_minor,
            client_patch
        ) {
            log::warn!("Invalid version header: {e}");
            return;
        }
        if SERVER_MAJOR != client_major {
            log::warn!("Invalid version: expected MAJOR {SERVER_MAJOR}, found {client_major}");
            accept_connection = false;
        }
        if SERVER_MAJOR == 0 && SERVER_MINOR != client_minor {
            log::warn!("Invalid version: expected MINOR {SERVER_MINOR}, found {client_minor}");
            accept_connection = false;
        }

        if !accept_connection {
            let s = format!("WinMount reject v{SERVER_MAJOR}.{SERVER_MINOR}.{SERVER_PATCH}");
            let _ = socket.send(s.into()).await;
            return;
        }
        // Send server version to client
        log::trace!(
            "Accepting a new client with version {client_major}.{client_minor}.{client_patch}"
        );
        let s = format!("WinMount accept v{SERVER_MAJOR}.{SERVER_MINOR}.{SERVER_PATCH}");
        if let Err(e) = socket.send(Message::Text(s)).await {
            log::warn!("Send accept connection failed: {e}");
            return;
        }
    } else {
        log::warn!("Failed to check version for incoming WebSocket connection");
        return;
    }

    // Handshake completed, enter main loop
    log::trace!("Enter WebSocket main loop");

    let mut fs_with_child_ctx = WsFileSystemContext::new(&app_ctx);

    while let Some(msg) = socket.recv().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                log::debug!("WebSocket disconnected: {e}");
                break;
            }
        };
        let msg = match msg {
            Message::Text(s) => s,
            Message::Binary(mut b) => {
                // Handle binary messages
                let resp = match handle_websocket_bin_request(
                    socket,
                    &app_ctx,
                    &mut fs_with_child_ctx,
                    &mut b,
                )
                .await
                {
                    Ok(m) => m,
                    Err(e) => WsBinMessage::Failure {
                        code: -1,
                        msg: e.to_string(),
                    },
                };
                if let Err(e) = handle_websocket_send_bin_msg(socket, resp).await {
                    log::warn!("Send response to client failed: {e}");
                    break;
                }
                continue;
            }
            _ => continue,
        };
        let msg: WsMessage = match serde_json::from_str(&msg) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("WebSocket recv got unexpected JSON: {e}");
                let msg = WsMessage::Failure {
                    code: -1,
                    msg: e.to_string(),
                };
                if let Err(e) = handle_websocket_send_msg(socket, msg).await {
                    log::warn!("Send response to client failed: {e}");
                    break;
                }
                continue;
            }
        };
        match msg {
            WsMessage::Request {
                syn,
                method,
                params,
            } => {
                if method == "close-current-session" {
                    // Close current WebSocket connection
                    let msg = WsMessage::new_resp_ok(syn, serde_json::Value::Null);
                    if let Err(e) = handle_websocket_send_msg(socket, msg).await {
                        log::warn!("Send response to client failed: {e}");
                        break;
                    }
                    break;
                }
                let resp = match handle_websocket_request(
                    socket,
                    &app_ctx,
                    &mut fs_with_child_ctx,
                    syn,
                    method,
                    params,
                )
                .await
                {
                    Ok(m) => m,
                    Err(e) => WsMessage::new_resp_err(syn, -1, e.to_string()),
                };
                if let Err(e) = handle_websocket_send_msg(socket, resp).await {
                    log::warn!("Send response to client failed: {e}");
                    break;
                }
            }
            _ => {
                log::warn!("Unsupported message: {msg:?}");
            }
        }
    }

    log::trace!("Disconnecting from the client...");
}

async fn ws_handler(ws: WebSocketUpgrade, State(app_state): State<WebAppState>) -> Response {
    ws.on_upgrade(|mut socket| async {
        handle_websocket(&mut socket, app_state.app_ctx).await;
        // HACK: Detect error kind by string comparison
        match socket.close().await {
            Err(e) if e.to_string() != "Connection closed normally" => {
                log::warn!("Failed to close WebSocket: {e}");
            },
            _ => (),
        }
    })
}

#[derive(Clone)]
struct WebAppState {
    app_ctx: Arc<Mutex<crate::AppContext>>,
    shutdown_notify: Arc<tokio::sync::Notify>,
}
impl FromRef<WebAppState> for Arc<Mutex<crate::AppContext>> {
    fn from_ref(input: &WebAppState) -> Self {
        input.app_ctx.clone()
    }
}
impl FromRef<WebAppState> for Arc<tokio::sync::Notify> {
    fn from_ref(input: &WebAppState) -> Self {
        input.shutdown_notify.clone()
    }
}

pub(super) fn main_service(
    app_ctx: Arc<Mutex<crate::AppContext>>,
    shutdown_notify: Arc<tokio::sync::Notify>,
) -> axum::Router {
    use axum::routing::get;
    let app = axum::Router::new()
        .route(
            "/",
            get(|| async { concat!("WinMountCore v", env!("CARGO_PKG_VERSION"), " daemon") }),
        )
        .route(
            "/api/shutdown",
            get({
                let shutdown_notify = shutdown_notify.clone();
                || async move { shutdown_notify.notify_one() }
            }),
        )
        .route("/ws", get(ws_handler))
        .with_state(WebAppState {
            app_ctx,
            shutdown_notify,
        });
    app
}
