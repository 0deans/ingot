//! All frontend notifications share one explicit, event-only namespace.
use crate::minecraft::launcher::{InstanceStatusEvent, LaunchProgressEvent};
use crate::running::{LeftoverServer, QuitRequest};
use crate::server::{ServerLogEvent, ServerStatusEvent};
use crate::system::MemorySettings;
use crate::version_change::crash::VersionChangeCrash;

#[taurpc::procedures(path = "events")]
pub(crate) trait EventsApi {
    /// The first start after a version change crashed
    #[taurpc(event)]
    async fn on_version_change_crash(event: VersionChangeCrash);

    /// Closing Ingot was asked for while servers run: the user picks what happens
    #[taurpc(event)]
    async fn on_quit_requested(event: QuitRequest);

    /// A start ran into servers left running from before Ingot closed
    #[taurpc(event)]
    async fn on_leftover_servers(servers: Vec<LeftoverServer>);

    #[taurpc(event)]
    async fn on_memory_changed(settings: MemorySettings);

    #[taurpc(event)]
    async fn on_instance_status_changed(event: InstanceStatusEvent);

    #[taurpc(event)]
    async fn on_launch_progress(event: LaunchProgressEvent);

    #[taurpc(event)]
    async fn on_server_log(event: ServerLogEvent);

    #[taurpc(event)]
    async fn on_server_status_changed(event: ServerStatusEvent);
}

#[derive(Clone)]
pub(crate) struct EventsApiImpl;

#[taurpc::resolvers]
impl EventsApi for EventsApiImpl {}
