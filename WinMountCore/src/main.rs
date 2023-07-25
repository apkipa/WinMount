mod util;

mod fs_provider;
mod fs_server;

const DEFAULT_DAEMON_PORT: u16 = 19423;

// TODO: Fix too old dependencies

use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::{Hash, Hasher},
    os::windows::io::AsRawHandle,
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        Arc, Mutex, RwLock, Weak,
    },
    time::SystemTime,
};

// use clap::{App, Arg};
use uuid::Uuid;

type FsProviders = HashMap<Uuid, Box<dyn fs_provider::FsProvider>>;
type FsServerProviders = HashMap<Uuid, Box<dyn fs_server::FsServerProvider>>;

fn run_loop(
    fs_providers: &mut FsProviders,
    fs_server_providers: &mut FsServerProviders,
) -> anyhow::Result<()> {
    log::warn!("Starting WinMountCore daemon...");

    // TODO: Main loop
    let memfs_provider = fs_providers
        .get(&fs_provider::memfs::MEMFS_ID)
        .unwrap()
        .as_ref();
    let memfs = memfs_provider.construct("")?;
    // let memfs = Arc::from(memfs);
    let dokan_server_provider = fs_server_providers
        .get(&fs_server::dokan::DOKAN_FSERVER_ID)
        .unwrap()
        .as_ref();
    let dokan_server = dokan_server_provider.construct(memfs)?;

    let shutdown_flag = Arc::new(AtomicU32::new(0));
    let shutdown_flag2 = Arc::clone(&shutdown_flag);
    ctrlc::set_handler(move || {
        shutdown_flag2.store(1, Ordering::Release);
        atomic_wait::wake_one(shutdown_flag2.as_ref());
    })?;
    util::real_wait(&shutdown_flag, 0);

    log::warn!("Stopping WinMountCore daemon...");

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

    let mut fs_providers: HashMap<Uuid, Box<dyn fs_provider::FsProvider>> = HashMap::new();
    fs_provider::init_fs_providers(|id, p| {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        match fs_providers.entry(id) {
            Occupied(_) => anyhow::bail!("filesystem id was already taken"),
            Vacant(e) => {
                e.insert(p);
                Ok(())
            }
        }
    })?;

    let mut fs_server_providers: HashMap<Uuid, Box<dyn fs_server::FsServerProvider>> =
        HashMap::new();
    fs_server::init_fs_server_providers(|id, p| {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        match fs_server_providers.entry(id) {
            Occupied(_) => anyhow::bail!("filesystem server id was already taken"),
            Vacant(e) => {
                e.insert(p);
                Ok(())
            }
        }
    })?;

    let result = run_loop(&mut fs_providers, &mut fs_server_providers);

    fs_server::uninit_fs_server_providers();
    fs_provider::uninit_fs_providers();

    result
}
