mod util;

mod client;
mod web;

mod fs_provider;
mod fs_server;

// TODO: Use WAMP protocol / jsonrpc for RPC & PubSub?

const DEFAULT_DAEMON_PORT: u16 = 19423;

use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
};

use anyhow::Context;
use clap::Parser;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::fs_provider::FileSystemCreationContext;

fn init_log(verbose: u32) -> anyhow::Result<()> {
    fn logger(
        write: &mut dyn std::io::Write,
        now: &mut flexi_logger::DeferredNow,
        record: &log::Record<'_>,
    ) -> Result<(), std::io::Error> {
        let level = record.level();
        let level_str = match level {
            flexi_logger::Level::Debug => "DEBG".to_string(),
            x => x.to_string(),
        };
        let styler = flexi_logger::style(level);
        write!(
            write,
            "[{}] {} [{}:{}] {}",
            styler.paint(
                now.now()
                    .naive_local()
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            ),
            styler.paint(level_str),
            record.file().unwrap_or("<unnamed>"),
            record.line().unwrap_or(0),
            &record.args()
        )
    }

    let log_spec = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        3.. => "trace",
    };
    flexi_logger::Logger::try_with_str(log_spec)?
        .set_palette("196;208;158;248;240".to_owned())
        .format(logger)
        .start()?;
    Ok(())
}

struct FSInfo {
    name: String,
    kind_id: Uuid,
    handler: Option<Arc<dyn fs_provider::FileSystemHandler>>,
    config: serde_json::Value,
}
impl FSInfo {
    fn new(name: String, kind_id: Uuid) -> Self {
        Self {
            name,
            kind_id,
            handler: None,
            config: serde_json::Value::Null,
        }
    }
}

struct FServerInfo {
    name: String,
    kind_id: Uuid,
    in_fs_id: Uuid,
    server: Option<Arc<dyn fs_server::FileSystemServer>>,
    config: serde_json::Value,
}
impl FServerInfo {
    fn new(name: String, kind_id: Uuid, in_fs_id: Uuid) -> Self {
        Self {
            name,
            kind_id,
            in_fs_id,
            server: None,
            config: serde_json::Value::Null,
        }
    }
}

struct AppContext {
    fs_providers: HashMap<Uuid, Box<dyn fs_provider::FsProvider>>,
    fs_server_providers: HashMap<Uuid, Box<dyn fs_server::FsServerProvider>>,
    filesystems: HashMap<Uuid, FSInfo>,
    filesystem_servers: HashMap<Uuid, FServerInfo>,
}
impl AppContext {
    fn new() -> Self {
        Self {
            fs_providers: HashMap::new(),
            fs_server_providers: HashMap::new(),
            filesystems: HashMap::new(),
            filesystem_servers: HashMap::new(),
        }
    }
}

// Public APIs (for general RPC purpose)
#[derive(Serialize, Deserialize)]
struct ListFileSystemItemData {
    id: Uuid,
    name: String,
    kind_id: Uuid,
    is_running: bool,
}
#[derive(Serialize, Deserialize)]
struct ListFileSystemProviderItemData {
    id: Uuid,
    name: String,
}
#[derive(Serialize, Deserialize)]
struct ListFServerItemData {
    id: Uuid,
    name: String,
    kind_id: Uuid,
    in_fs_id: Uuid,
    is_running: bool,
}
#[derive(Serialize, Deserialize)]
struct ListFServerProviderItemData {
    id: Uuid,
    name: String,
}
#[derive(Serialize, Deserialize)]
struct GetFileSystemInfoData {
    name: String,
    kind_id: Uuid,
    is_running: bool,
    config: serde_json::Value,
}
#[derive(Serialize, Deserialize)]
struct GetFServerInfoData {
    name: String,
    kind_id: Uuid,
    in_fs_id: Uuid,
    is_running: bool,
    config: serde_json::Value,
}
impl AppContext {
    fn create_fs(
        &mut self,
        name: String,
        kind_id: Uuid,
        config: serde_json::Value,
    ) -> anyhow::Result<Uuid> {
        use std::collections::hash_map::Entry::*;
        let fs_info = FSInfo {
            name,
            kind_id,
            handler: None,
            config,
        };
        Ok(loop {
            let id = Uuid::new_v4();
            match self.filesystems.entry(id) {
                Occupied(_) => continue,
                Vacant(e) => {
                    e.insert(fs_info);
                }
            }
            break id;
        })
    }
    fn remove_fs(&mut self, id: Uuid) -> anyhow::Result<()> {
        use std::collections::hash_map::Entry::*;
        match self.filesystems.entry(id) {
            Occupied(e) => {
                let fs = e.get();
                if fs.handler.is_some() {
                    anyhow::bail!("cannot remove a running filesystem");
                }
                e.remove();
            }
            Vacant(_) => anyhow::bail!("filesystem not found"),
        }
        Ok(())
    }
    fn start_fs(&mut self, id: Uuid) -> anyhow::Result<bool> {
        let fs_info = self
            .filesystems
            .get_mut(&id)
            .context("filesystem not found")?;
        Ok(if fs_info.handler.is_none() {
            // TODO: Optimize performance
            AppContextForCreation::from(self).get_or_run_fs(&id, "")?;
            true
        } else {
            false
        })
    }
    fn stop_fs(&mut self, id: Uuid) -> anyhow::Result<bool> {
        let fs_info = self
            .filesystems
            .get_mut(&id)
            .context("filesystem not found")?;
        Ok(if let Some(handler) = &mut fs_info.handler {
            if Arc::get_mut(handler).is_none() {
                anyhow::bail!("filesystem is still being used by other components");
            }
            fs_info.handler = None;
            true
        } else {
            false
        })
    }
    fn create_fsrv(
        &mut self,
        name: String,
        kind_id: Uuid,
        in_fs_id: Uuid,
        config: serde_json::Value,
    ) -> anyhow::Result<Uuid> {
        use std::collections::hash_map::Entry::*;
        let fsrv_info = FServerInfo {
            name,
            kind_id,
            in_fs_id,
            server: None,
            config,
        };
        Ok(loop {
            let id = Uuid::new_v4();
            match self.filesystem_servers.entry(id) {
                Occupied(_) => continue,
                Vacant(e) => {
                    e.insert(fsrv_info);
                }
            }
            break id;
        })
    }
    fn remove_fsrv(&mut self, id: Uuid) -> anyhow::Result<()> {
        use std::collections::hash_map::Entry::*;
        match self.filesystem_servers.entry(id) {
            Occupied(e) => {
                let fs = e.get();
                if fs.server.is_some() {
                    anyhow::bail!("cannot remove a running filesystem server");
                }
                e.remove();
            }
            Vacant(_) => anyhow::bail!("filesystem server not found"),
        }
        Ok(())
    }
    fn start_fsrv(&mut self, id: Uuid) -> anyhow::Result<bool> {
        let fsrv_info = self
            .filesystem_servers
            .get_mut(&id)
            .context("filesystem server not found")?;
        Ok(if fsrv_info.server.is_none() {
            AppContextForCreation::from(self).start_fs_server(&id)?;
            true
        } else {
            false
        })
    }
    fn stop_fsrv(&mut self, id: Uuid) -> anyhow::Result<bool> {
        let fsrv_info = self
            .filesystem_servers
            .get_mut(&id)
            .context("filesystem server not found")?;
        Ok(if let Some(server) = &mut fsrv_info.server {
            if Arc::get_mut(server).is_none() {
                anyhow::bail!("filesystem server is still being used by other components");
            }
            fsrv_info.server = None;
            true
        } else {
            false
        })
    }
    fn list_fs(&mut self) -> anyhow::Result<Vec<ListFileSystemItemData>> {
        Ok(self
            .filesystems
            .iter()
            .map(|(id, fs_info)| ListFileSystemItemData {
                id: *id,
                kind_id: fs_info.kind_id,
                name: fs_info.name.clone(),
                is_running: fs_info.handler.is_some(),
            })
            .collect())
    }
    fn list_fsp(&mut self) -> anyhow::Result<Vec<ListFileSystemProviderItemData>> {
        Ok(self
            .fs_providers
            .iter()
            .map(|(id, fsp)| ListFileSystemProviderItemData {
                id: *id,
                name: fsp.get_name().to_owned(),
            })
            .collect())
    }
    fn list_fsrv(&mut self) -> anyhow::Result<Vec<ListFServerItemData>> {
        Ok(self
            .filesystem_servers
            .iter()
            .map(|(id, fsrv_info)| ListFServerItemData {
                id: *id,
                name: fsrv_info.name.clone(),
                kind_id: fsrv_info.kind_id,
                in_fs_id: fsrv_info.in_fs_id,
                is_running: fsrv_info.server.is_some(),
            })
            .collect())
    }
    fn list_fsrvp(&mut self) -> anyhow::Result<Vec<ListFServerProviderItemData>> {
        Ok(self
            .fs_server_providers
            .iter()
            .map(|(id, fsrvp)| ListFServerProviderItemData {
                id: *id,
                name: fsrvp.get_name().to_owned(),
            })
            .collect())
    }
    fn get_fs_info(&mut self, id: Uuid) -> anyhow::Result<GetFileSystemInfoData> {
        let fs_info = self.filesystems.get(&id).context("filesystem not found")?;
        Ok(GetFileSystemInfoData {
            name: fs_info.name.clone(),
            kind_id: fs_info.kind_id,
            is_running: fs_info.handler.is_some(),
            config: fs_info.config.clone(),
        })
    }
    fn get_fsrv_info(&mut self, id: Uuid) -> anyhow::Result<GetFServerInfoData> {
        let fsrv_info = self
            .filesystem_servers
            .get(&id)
            .context("filesystem not found")?;
        Ok(GetFServerInfoData {
            name: fsrv_info.name.clone(),
            kind_id: fsrv_info.kind_id,
            in_fs_id: fsrv_info.in_fs_id,
            is_running: fsrv_info.server.is_some(),
            config: fsrv_info.config.clone(),
        })
    }
    fn update_fs_info(
        &mut self,
        id: Uuid,
        name: Option<String>,
        config: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        let fs_info = self
            .filesystems
            .get_mut(&id)
            .context("filesystem not found")?;
        if let Some(name) = name {
            fs_info.name = name;
        }
        if let Some(config) = config {
            fs_info.config = config;
        }
        Ok(())
    }
    fn update_fsrv_info(
        &mut self,
        id: Uuid,
        name: Option<String>,
        config: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        let fsrv_info = self
            .filesystem_servers
            .get_mut(&id)
            .context("filesystem server not found")?;
        if let Some(name) = name {
            fsrv_info.name = name;
        }
        if let Some(config) = config {
            fsrv_info.config = config;
        }
        Ok(())
    }
}

struct AppContextForCreation<'a> {
    fs_providers: &'a HashMap<Uuid, Box<dyn fs_provider::FsProvider>>,
    fs_server_providers: &'a HashMap<Uuid, Box<dyn fs_server::FsServerProvider>>,
    filesystems: &'a mut HashMap<Uuid, FSInfo>,
    filesystem_servers: &'a mut HashMap<Uuid, FServerInfo>,
    creation_dep_path: HashSet<Uuid>,
}

impl AppContextForCreation<'_> {
    fn start_fs_server(&mut self, id: &Uuid) -> anyhow::Result<()> {
        let fserver_info = self
            .filesystem_servers
            .get_mut(id)
            .context("filesystem server not found")?;
        if fserver_info.server.is_some() {
            return Ok(());
        }
        let fserver_provider = self
            .fs_server_providers
            .get(&fserver_info.kind_id)
            .context("filesystem server provider not found")?;
        let in_fs = self
            .filesystems
            .get(&fserver_info.in_fs_id)
            .context("input filesystem not found")?;
        let in_fs = Arc::clone(
            in_fs
                .handler
                .as_ref()
                .context("input filesystem not created")?,
        );
        fserver_info.server = Some(fserver_provider.construct(in_fs, fserver_info.config.clone())?);
        Ok(())
    }
}

impl<'a> From<&'a mut AppContext> for AppContextForCreation<'a> {
    fn from(value: &'a mut AppContext) -> Self {
        Self {
            fs_providers: &value.fs_providers,
            fs_server_providers: &value.fs_server_providers,
            filesystems: &mut value.filesystems,
            filesystem_servers: &mut value.filesystem_servers,
            creation_dep_path: HashSet::new(),
        }
    }
}

impl FileSystemCreationContext for AppContextForCreation<'_> {
    fn get_or_run_fs(
        &mut self,
        id: &Uuid,
        prefix_path: &str,
    ) -> Result<Arc<dyn fs_provider::FileSystemHandler>, fs_provider::FileSystemCreationError> {
        // TODO: prefix_path wrapper
        use fs_provider::FileSystemCreationError;
        if !self.creation_dep_path.insert(*id) {
            return Err(FileSystemCreationError::CyclicDependency);
        }
        if !prefix_path.is_empty() {
            todo!("implement prefix_path wrapper");
        }
        let mut this = scopeguard::guard(self, |this| {
            this.creation_dep_path.remove(id);
        });
        let ref mut this = *this;
        let mut fs_info = this
            .filesystems
            .get_mut(id)
            .ok_or(FileSystemCreationError::NotFound)?;
        let fs_handler = if let Some(h) = &fs_info.handler {
            Arc::clone(h)
        } else {
            let fs_provider = this
                .fs_providers
                .get(&fs_info.kind_id)
                .ok_or(FileSystemCreationError::InvalidFileSystem)?;
            let fs_handler = fs_provider.construct(fs_info.config.clone(), *this)?;
            // drop(fs_info);
            fs_info = this
                .filesystems
                .get_mut(id)
                .ok_or(FileSystemCreationError::NotFound)?;
            fs_info.handler = Some(Arc::clone(&fs_handler));
            fs_handler
        };
        Ok(fs_handler)
    }
}

async fn shutdown_signal(shutdown_notify: &tokio::sync::Notify) {
    tokio::select! {
        r = tokio::signal::ctrl_c() => {
            r.expect("expects shutdown signal handler");
            println!("Ctrl+C");
        },
        _ = shutdown_notify.notified() => (),
    }
}

async fn run_loop(mut app_ctx: AppContext, server_addr: SocketAddr) -> anyhow::Result<()> {
    log::warn!("Starting WinMountCore daemon...");

    log::info!("Now listening on {server_addr}");
    let shutdown_notify = Arc::new(tokio::sync::Notify::new());
    let app_ctx = Arc::new(Mutex::new(app_ctx));
    axum::Server::bind(&server_addr)
        .serve(
            web::main_service(Arc::clone(&app_ctx), Arc::clone(&shutdown_notify))
                .into_make_service(),
        )
        .with_graceful_shutdown(shutdown_signal(&shutdown_notify))
        .await?;

    log::warn!("Stopping WinMountCore daemon...");

    let mut app_ctx = Arc::into_inner(app_ctx)
        .expect("app_ctx is still being shared")
        .into_inner()
        .unwrap();
    app_ctx.filesystem_servers.clear();
    app_ctx.filesystems.clear();

    Ok(())
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct AppCli {
    #[command(subcommand)]
    command: AppCommands,
}

#[derive(clap::Subcommand)]
enum AppCommands {
    /// Starts a long-running background service
    Daemon {
        /// The location where the configuration file lives
        #[arg(long)]
        config: Option<String>,
        /// Level of verbosity
        #[arg(short, long, action = clap::ArgAction::Count)]
        verbose: u8,
    },
    /// Stops the running background service
    StopDaemon {},
    /// Lists filesystems or show details
    ListFs {
        #[arg(short, long)]
        id: Option<Uuid>,
    },
    /// Lists filesystem providers or show details
    ListFsp {
        #[arg(short, long)]
        id: Option<Uuid>,
    },
    /// Lists filesystem servers or show details
    ListFsrv {
        #[arg(short, long)]
        id: Option<Uuid>,
    },
    /// Lists filesystem server providers or show details
    ListFsrvp {
        #[arg(short, long)]
        id: Option<Uuid>,
    },
    /// Creates a new filesystem
    CreateFs {
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        provider: Uuid,
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Creates a new filesystem server
    CreateFsrv {
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        provider: Uuid,
        #[arg(short, long)]
        input_fs: Uuid,
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Deletes an existing filesystem
    RemoveFs {
        #[arg(short, long)]
        id: Uuid,
    },
    /// Deletes an existing filesystem server
    RemoveFsrv {
        #[arg(short, long)]
        id: Uuid,
    },
    /// Starts an existing filesystem
    StartFs {
        #[arg(short, long)]
        id: Uuid,
    },
    /// Starts an existing filesystem server
    StartFsrv {
        #[arg(short, long)]
        id: Uuid,
    },
    /// Stops an existing filesystem
    StopFs {
        #[arg(short, long)]
        id: Uuid,
    },
    /// Stops an existing filesystem server
    StopFsrv {
        #[arg(short, long)]
        id: Uuid,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = AppCli::parse();

    // TODO: Support config files, log levels, ...

    // TODO: Specify port in config file (?)
    let daemon_port = std::env::var("WINMOUNT_DAEMON_PORT")
        .ok()
        .and_then(|x| x.parse::<u16>().ok())
        .unwrap_or(DEFAULT_DAEMON_PORT);

    let socket_addr: IpAddr = "127.0.0.1".parse()?;
    let socket_addr = SocketAddr::new(socket_addr, daemon_port);

    match cli.command {
        AppCommands::Daemon { config, verbose } => {
            // init_log(3)?;
            init_log(verbose.into())?;

            let mut app_ctx = AppContext::new();

            fs_provider::init_fs_providers(|id, p| {
                use std::collections::hash_map::Entry::{Occupied, Vacant};
                match app_ctx.fs_providers.entry(id) {
                    Occupied(_) => anyhow::bail!("filesystem provider id was already taken"),
                    Vacant(e) => {
                        e.insert(p);
                        Ok(())
                    }
                }
            })?;
            scopeguard::defer! {
                fs_provider::uninit_fs_providers();
            }

            fs_server::init_fs_server_providers(|id, p| {
                use std::collections::hash_map::Entry::{Occupied, Vacant};
                match app_ctx.fs_server_providers.entry(id) {
                    Occupied(_) => anyhow::bail!("filesystem server provider id was already taken"),
                    Vacant(e) => {
                        e.insert(p);
                        Ok(())
                    }
                }
            })?;
            scopeguard::defer! {
                fs_server::uninit_fs_server_providers();
            }

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(run_loop(app_ctx, socket_addr))?;
        }
        _ => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(client::handle_client_cli(&cli, socket_addr))?;
        }
    }

    // {
    //     let rt = tokio::runtime::Builder::new_current_thread()
    //         .enable_all()
    //         .build()?;
    //     rt.block_on(run_loop(app_ctx, socket_addr))?;
    // }
    // drop(app_ctx);

    Ok(())
}
