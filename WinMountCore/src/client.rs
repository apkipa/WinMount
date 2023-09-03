use std::sync::atomic::AtomicU64;
use std::{net::SocketAddr, process::Stdio};

use anyhow::Context;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppCommands;

use crate::util::parse_u32;
const CLIENT_MAJOR: u32 = parse_u32(env!("CARGO_PKG_VERSION_MAJOR"));
const CLIENT_MINOR: u32 = parse_u32(env!("CARGO_PKG_VERSION_MINOR"));
const CLIENT_PATCH: u32 = parse_u32(env!("CARGO_PKG_VERSION_PATCH"));

// TODO: Fix inaccurate result?
fn is_external_daemon_running(server_addr: SocketAddr) -> bool {
    use netstat::*;
    let mut it = match iterate_sockets_info(AddressFamilyFlags::IPV4, ProtocolFlags::TCP) {
        Ok(it) => it,
        Err(_) => return false,
    };
    it.any(|v| {
        use ProtocolSocketInfo::*;
        matches!(v, Ok(SocketInfo { protocol_socket_info: Tcp(info), .. }) if info.local_port == server_addr.port())
    })
    // for i in it {
    //     let info = match i {
    //         Ok(v) => v,
    //         Err(_) => continue,
    //     };
    //     let info = match info.protocol_socket_info {
    //         ProtocolSocketInfo::Tcp(info) => info,
    //         ProtocolSocketInfo::Udp(_) => continue,
    //     };
    //     if info.local_port == server_addr.port() {
    //         return true;
    //     }
    // }
    // return false;
}

type WebSocketStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn connect_daemon_ws(server_addr: &str) -> anyhow::Result<WebSocketStream> {
    let server_addr = format!("ws://{server_addr}/ws");
    let (wss, _) = tokio_tungstenite::connect_async(server_addr).await?;
    Ok(wss)
}

async fn start_external_daemon() -> anyhow::Result<()> {
    let cur_exe = std::env::current_exe()?;
    std::process::Command::new(cur_exe)
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

async fn stop_external_daemon(server_addr: &str) -> anyhow::Result<()> {
    let server_addr = format!("http://{server_addr}/api/shutdown");
    let client = hyper::client::Client::new();
    client.get(server_addr.parse()?).await?;
    Ok(())
}

async fn ws_send_request(
    ws: &mut WebSocketStream,
    method: &str,
    params: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    // TODO: Optimize logic
    use tokio_tungstenite::tungstenite::Message;
    static SYN_COUNTER: AtomicU64 = AtomicU64::new(0);
    let client_syn = SYN_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let msg = serde_json::json!({
        "type": "request",
        "syn": client_syn,
        "method": method.to_string(),
        "params": params,
    });
    ws.send(Message::text(msg.to_string())).await?;
    let resp = ws.next().await.context("cannot fetch response")??;
    let resp = match resp {
        Message::Text(resp) => resp,
        _ => anyhow::bail!("incorrect WebSocket message type"),
    };
    let resp: crate::web::WsMessage = serde_json::from_str(&resp)?;
    let resp = match resp {
        crate::web::WsMessage::Response {
            syn,
            code,
            msg,
            data,
        } if syn == client_syn => {
            if code == 0 {
                data
            } else {
                anyhow::bail!("RPC failed with code {code}: {msg}");
            }
        }
        _ => anyhow::bail!("incorrect response, got {resp:?}"),
    };
    Ok(resp)
}

pub(super) async fn handle_client_cli(
    cli: &super::AppCli,
    server_addr: SocketAddr,
) -> anyhow::Result<()> {
    let server_addr_str = server_addr.to_string();
    let port = server_addr.port();
    let connect_or_start = || async {
        use tokio_tungstenite::tungstenite::Message;
        let mut ws = match connect_daemon_ws(&server_addr_str).await {
            Ok(ws) => ws,
            _ => {
                println!("* Daemon not running; starting now at port {port}");
                start_external_daemon().await?;
                println!("* Daemon started successfully");
                connect_daemon_ws(&server_addr_str).await?
            }
        };
        let handshake_str =
            format!("WinMount connect v{CLIENT_MAJOR}.{CLIENT_MINOR}.{CLIENT_PATCH}");
        ws.send(Message::text(handshake_str)).await?;
        let msg = ws.next().await.context("cannot fetch response")??;
        let handshake_success = match msg {
            Message::Text(s) => s.starts_with("WinMount accept v"),
            _ => false,
        };
        if handshake_success {
            Ok(ws)
        } else {
            anyhow::bail!("handshake with daemon failed");
        }
    };
    if let AppCommands::StopDaemon {} = cli.command {
        stop_external_daemon(&server_addr_str).await?;
        return Ok(());
    }

    let mut ws = connect_or_start().await?;
    match &cli.command {
        AppCommands::ListFs { id: Some(id) } => {
            let params = serde_json::json!({ "id": id });
            let resp = ws_send_request(&mut ws, "get-fs-info", params).await?;
            let resp: crate::GetFileSystemInfoData = serde_json::from_value(resp)?;
            println!("Name: {}", resp.name);
            println!("Kind Id: {}", resp.kind_id);
            println!("Is Running: {}", resp.is_running);
            println!("Config: {}", resp.config);
        }
        AppCommands::ListFs { id: None } => {
            #[derive(Deserialize)]
            struct Response {
                fs_list: Vec<crate::ListFileSystemItemData>,
            }
            let params = serde_json::Value::Null;
            let resp = ws_send_request(&mut ws, "list-fs", params).await?;
            let resp: Response = serde_json::from_value(resp)?;
            println!("Id | Name | Kind Id | Is Running | Is Global");
            for i in resp.fs_list {
                println!(
                    "{} | {} | {} | {} | {}",
                    i.id, i.name, i.kind_id, i.is_running, i.is_global
                );
            }
        }
        AppCommands::ListFsp { id: Some(id) } => {
            #[derive(Deserialize)]
            struct Response {
                fsp_list: Vec<crate::ListFileSystemProviderItemData>,
            }
            let params = serde_json::Value::Null;
            let resp = ws_send_request(&mut ws, "list-fsp", params).await?;
            let resp: Response = serde_json::from_value(resp)?;
            let info = resp
                .fsp_list
                .into_iter()
                .filter(|v| v.id == *id)
                .next()
                .context("filesystem provider not found")?;
            println!("Name: {}", info.name);
        }
        AppCommands::ListFsp { id: None } => {
            #[derive(Deserialize)]
            struct Response {
                fsp_list: Vec<crate::ListFileSystemProviderItemData>,
            }
            let params = serde_json::Value::Null;
            let resp = ws_send_request(&mut ws, "list-fsp", params).await?;
            let resp: Response = serde_json::from_value(resp)?;
            println!("Id | Name");
            for i in resp.fsp_list {
                println!("{} | {}", i.id, i.name);
            }
        }
        AppCommands::ListFsrv { id: Some(id) } => {
            let params = serde_json::json!({ "id": id });
            let resp = ws_send_request(&mut ws, "get-fsrv-info", params).await?;
            let resp: crate::GetFServerInfoData = serde_json::from_value(resp)?;
            println!("Name: {}", resp.name);
            println!("Kind Id: {}", resp.kind_id);
            println!("Is Running: {}", resp.is_running);
            println!("Input Filesystem Id: {}", resp.in_fs_id);
            println!("Config: {}", resp.config);
        }
        AppCommands::ListFsrv { id: None } => {
            #[derive(Deserialize)]
            struct Response {
                fsrv_list: Vec<crate::ListFServerItemData>,
            }
            let params = serde_json::Value::Null;
            let resp = ws_send_request(&mut ws, "list-fsrv", params).await?;
            let resp: Response = serde_json::from_value(resp)?;
            println!("Id | Name | Kind Id | Input Fs Id | Is Running");
            for i in resp.fsrv_list {
                println!(
                    "{} | {} | {} | {} | {}",
                    i.id, i.name, i.kind_id, i.in_fs_id, i.is_running
                );
            }
        }
        AppCommands::ListFsrvp { id: Some(id) } => {
            #[derive(Deserialize)]
            struct Response {
                fsrvp_list: Vec<crate::ListFServerProviderItemData>,
            }
            let params = serde_json::Value::Null;
            let resp = ws_send_request(&mut ws, "list-fsrvp", params).await?;
            let resp: Response = serde_json::from_value(resp)?;
            let info = resp
                .fsrvp_list
                .into_iter()
                .filter(|v| v.id == *id)
                .next()
                .context("filesystem server provider not found")?;
            println!("Name: {}", info.name);
        }
        AppCommands::ListFsrvp { id: None } => {
            #[derive(Deserialize)]
            struct Response {
                fsrvp_list: Vec<crate::ListFServerProviderItemData>,
            }
            let params = serde_json::Value::Null;
            let resp = ws_send_request(&mut ws, "list-fsrvp", params).await?;
            let resp: Response = serde_json::from_value(resp)?;
            println!("Id | Name");
            for i in resp.fsrvp_list {
                println!("{} | {}", i.id, i.name);
            }
        }
        AppCommands::CreateFs {
            name,
            provider,
            config,
        } => {
            #[derive(Deserialize)]
            struct Response {
                fs_id: Uuid,
            }
            let params = if let Some(config) = config {
                serde_json::json!({
                    "name": name,
                    "kind_id": provider,
                    "config": serde_json::from_str::<serde_json::Value>(config)?,
                })
            } else {
                serde_json::json!({
                    "name": name,
                    "kind_id": provider,
                })
            };
            let resp = ws_send_request(&mut ws, "create-fs", params).await?;
            let resp: Response = serde_json::from_value(resp)?;
            println!("Created a new filesystem with id {}", resp.fs_id);
        }
        AppCommands::CreateFsrv {
            name,
            provider,
            input_fs,
            config,
        } => {
            #[derive(Deserialize)]
            struct Response {
                fsrv_id: Uuid,
            }
            let params = if let Some(config) = config {
                serde_json::json!({
                    "name": name,
                    "kind_id": provider,
                    "in_fs_id": input_fs,
                    "config": serde_json::from_str::<serde_json::Value>(config)?,
                })
            } else {
                serde_json::json!({
                    "name": name,
                    "kind_id": provider,
                    "in_fs_id": input_fs,
                })
            };
            let resp = ws_send_request(&mut ws, "create-fsrv", params).await?;
            let resp: Response = serde_json::from_value(resp)?;
            println!("Created a new filesystem server with id {}", resp.fsrv_id);
        }
        AppCommands::RemoveFs { id } => {
            let params = serde_json::json!({ "id": id });
            let resp = ws_send_request(&mut ws, "remove-fs", params).await?;
        }
        AppCommands::RemoveFsrv { id } => {
            let params = serde_json::json!({ "id": id });
            let resp = ws_send_request(&mut ws, "remove-fsrv", params).await?;
        }
        AppCommands::StartFs { id } => {
            #[derive(Deserialize)]
            struct Response {
                new_started: bool,
            }
            let params = serde_json::json!({ "id": id });
            let resp = ws_send_request(&mut ws, "start-fs", params).await?;
            let resp: Response = serde_json::from_value(resp)?;
        }
        AppCommands::StartFsrv { id } => {
            #[derive(Deserialize)]
            struct Response {
                new_started: bool,
            }
            let params = serde_json::json!({ "id": id });
            let resp = ws_send_request(&mut ws, "start-fsrv", params).await?;
            let resp: Response = serde_json::from_value(resp)?;
        }
        AppCommands::StopFs { id } => {
            #[derive(Deserialize)]
            struct Response {
                new_stopped: bool,
            }
            let params = serde_json::json!({ "id": id });
            let resp = ws_send_request(&mut ws, "stop-fs", params).await?;
            let resp: Response = serde_json::from_value(resp)?;
        }
        AppCommands::StopFsrv { id } => {
            #[derive(Deserialize)]
            struct Response {
                new_stopped: bool,
            }
            let params = serde_json::json!({ "id": id });
            let resp = ws_send_request(&mut ws, "stop-fsrv", params).await?;
            let resp: Response = serde_json::from_value(resp)?;
        }
        _ => anyhow::bail!("supplied unsupported subcommand"),
    }
    ws.close(None).await?;
    Ok(())
}
