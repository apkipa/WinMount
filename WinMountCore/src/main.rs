mod util;

mod fs_provider;
mod fs_server;

const DEFAULT_DAEMON_PORT: u16 = 19423;

use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    os::windows::io::AsRawHandle,
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        Arc, Mutex, RwLock, Weak,
    },
    time::SystemTime,
};

use anyhow::Context;
// use clap::{App, Arg};
use uuid::Uuid;

use crate::fs_provider::FileSystemCreationContext;

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

struct AppContextForCreation<'a> {
    fs_providers: &'a HashMap<Uuid, Box<dyn fs_provider::FsProvider>>,
    fs_server_providers: &'a HashMap<Uuid, Box<dyn fs_server::FsServerProvider>>,
    filesystems: &'a mut HashMap<Uuid, FSInfo>,
    filesystem_servers: &'a mut HashMap<Uuid, FServerInfo>,
    creation_dep_path: HashSet<Uuid>,
}

impl AppContextForCreation<'_> {
    fn create_fs_server(&mut self, id: &Uuid) -> anyhow::Result<()> {
        let fserver_info = self
            .filesystem_servers
            .get_mut(id)
            .context("Filesystem server not found")?;
        if fserver_info.server.is_some() {
            return Ok(());
        }
        let fserver_provider = self
            .fs_server_providers
            .get(&fserver_info.kind_id)
            .context("Filesystem server provider not found")?;
        let in_fs = self
            .filesystems
            .get(&fserver_info.in_fs_id)
            .context("Input filesystem not found")?;
        let in_fs = Arc::clone(
            in_fs
                .handler
                .as_ref()
                .context("Input filesystem not created")?,
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
    fn get_or_create_fs(
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
            todo!("Implement prefix_path wrapper");
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

fn run_loop(app_ctx: &mut AppContext) -> anyhow::Result<()> {
    log::warn!("Starting WinMountCore daemon...");

    // TODO: Main loop
    let memfs_id = Uuid::new_v4();
    let dokan_fserver_id = Uuid::new_v4();
    app_ctx.filesystems.insert(
        memfs_id,
        FSInfo::new("First memfs".to_owned(), fs_provider::memfs::MEMFS_ID),
    );
    app_ctx.filesystem_servers.insert(
        dokan_fserver_id,
        FServerInfo::new(
            "First Dokan drive".to_owned(),
            fs_server::dokan::DOKAN_FSERVER_ID,
            memfs_id,
        ),
    );
    let memfs = AppContextForCreation::from(&mut *app_ctx).get_or_create_fs(&memfs_id, "")?;
    AppContextForCreation::from(&mut *app_ctx).create_fs_server(&dokan_fserver_id)?;
    // let memfs_provider = app_ctx
    //     .fs_providers
    //     .get(&fs_provider::memfs::MEMFS_ID)
    //     .unwrap()
    //     .as_ref();
    // let memfs = memfs_provider.construct(
    //     serde_json::json!({}),
    //     &mut AppContextForCreation::from(app_ctx),
    // )?;
    // let dokan_server_provider = app_ctx
    //     .fs_server_providers
    //     .get(&fs_server::dokan::DOKAN_FSERVER_ID)
    //     .unwrap()
    //     .as_ref();
    // let dokan_server = dokan_server_provider.construct(memfs)?;

    let shutdown_flag = Arc::new(AtomicU32::new(0));
    let shutdown_flag2 = Arc::clone(&shutdown_flag);
    ctrlc::set_handler(move || {
        shutdown_flag2.store(1, Ordering::Release);
        atomic_wait::wake_one(shutdown_flag2.as_ref());
    })?;
    util::real_wait(&shutdown_flag, 0);

    log::warn!("Stopping WinMountCore daemon...");

    app_ctx.filesystem_servers.clear();
    app_ctx.filesystems.clear();

    Ok(())
}

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

fn main() -> anyhow::Result<()> {
    init_log(3)?;

    // TODO: Sort out logic
    let daemon_port = std::env::var("WINMOUNT_DAEMON_PORT")
        .ok()
        .and_then(|x| x.parse::<u16>().ok())
        .unwrap_or(DEFAULT_DAEMON_PORT);

    // TODO: Remove this
    println!("Port is: {daemon_port}");

    let mut app_ctx = AppContext::new();

    fs_provider::init_fs_providers(|id, p| {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        match app_ctx.fs_providers.entry(id) {
            Occupied(_) => anyhow::bail!("filesystem id was already taken"),
            Vacant(e) => {
                e.insert(p);
                Ok(())
            }
        }
    })?;

    fs_server::init_fs_server_providers(|id, p| {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        match app_ctx.fs_server_providers.entry(id) {
            Occupied(_) => anyhow::bail!("filesystem server id was already taken"),
            Vacant(e) => {
                e.insert(p);
                Ok(())
            }
        }
    })?;

    let result = run_loop(&mut app_ctx);
    drop(app_ctx);

    fs_server::uninit_fs_server_providers();
    fs_provider::uninit_fs_providers();

    result
}
