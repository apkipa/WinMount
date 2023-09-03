pub mod dokan;

use std::sync::Arc;

use uuid::Uuid;

// NOTE: Drop FileSystemServer to stop a server, synchronously
pub trait FileSystemServer: Send + Sync {}

pub trait FsServerProvider: Send {
    fn get_id(&self) -> Uuid;
    fn get_name(&self) -> &'static str;
    // Follows SemVer
    fn get_version(&self) -> (u32, u32, u32);
    fn construct(
        &self,
        fs: Arc<dyn crate::fs_provider::FileSystemHandler>,
        config: serde_json::Value,
    ) -> anyhow::Result<Arc<dyn FileSystemServer>>;
    fn get_template_config(&self) -> serde_json::Value;
}

pub fn init_fs_server_providers(
    mut register_fn: impl FnMut(Uuid, Box<dyn FsServerProvider>) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut reg = |p: Box<dyn FsServerProvider>| register_fn(p.get_id(), p);
    reg(Box::new(dokan::DokanFServerProvider::new()))?;
    unsafe {
        dokan_sys::DokanInit();
    }
    Ok(())
}

pub fn uninit_fs_server_providers() {
    unsafe {
        dokan_sys::DokanShutdown();
    }
}
