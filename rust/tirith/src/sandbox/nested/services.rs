//! The system services a command may look up: those of Codex's own macOS policy. Codex's process keeps what OpenBot's profile lets it.
use super::sb::Sb;

const SERVICES: [&str; 24] = [
    "app.tirith.control", // by which the daemon tells this profile from a stranger's
    "com.apple.PowerManagement.control",
    "com.apple.analyticsd",
    "com.apple.analyticsd.messagetracer",
    "com.apple.appsleep",
    "com.apple.bsd.dirhelper",
    "com.apple.cfprefsd.agent",
    "com.apple.cfprefsd.daemon",
    "com.apple.diagnosticd",
    "com.apple.dt.automationmode.reader",
    "com.apple.espd",
    "com.apple.logd",
    "com.apple.logd.events",
    "com.apple.secinitd",
    "com.apple.system.DirectoryService.libinfo_v1",
    "com.apple.system.logger",
    "com.apple.system.notification_center",
    "com.apple.system.opendirectoryd.libinfo",
    "com.apple.system.opendirectoryd.membership",
    "com.apple.trustd",
    "com.apple.trustd.agent",
    "com.apple.xpc.activity.unmanaged",
    "com.apple.audio.audiohald",
    "com.apple.audio.AudioComponentRegistrar",
];
/// What that policy adds for commands that have the network.
const WITH_NETWORK: [&str; 6] = [
    "com.apple.SecurityServer",
    "com.apple.networkd",
    "com.apple.ocspd",
    "com.apple.SystemConfiguration.DNSConfiguration",
    "com.apple.SystemConfiguration.configd",
    "com.apple.TrustEvaluationAgent",
];

pub(super) fn lookups(sb: &mut Sb, net: bool, own: Option<&str>) {
    sb.line("(deny mach-lookup)");
    if let Some(own) = own {
        sb.line(&format!("(allow mach-lookup {own})"));
    }
    sb.line("(allow mach-lookup (local-name \"com.apple.cfprefsd.agent\"))");
    let more: &[&str] = if net { &WITH_NETWORK } else { &[] };
    for name in SERVICES.iter().chain(more) {
        sb.line(&format!("(allow mach-lookup (global-name \"{name}\"))"));
    }
}
