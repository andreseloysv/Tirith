//! What Codex's own sandbox refuses a command besides writing and the network; and the Codex program is no deputy of its commands.
use super::sb::Sb;

const REFUSED: [&str; 6] = [
    "(deny lsopen)",
    "(deny job-creation)",
    "(deny user-preference-write)",
    "(deny mach-priv-task-port)",
    "(deny signal)",
    "(allow signal (target same-sandbox))",
];
/// What Codex's process may and its commands may not, with the one case each that they may. (Not `sysctl-write`: shells read some of theirs as a write, and as this user it changes nothing.)
const ITS_OWN: [(&str, &str); 2] = [
    (
        "iokit-open",
        "(iokit-registry-entry-class \"RootDomainUserClient\")",
    ),
    ("mach-register", ""),
];

pub(super) fn limit(sb: &mut Sb, program: &str, own: Option<&str>) {
    REFUSED.iter().for_each(|rule| sb.line(rule));
    for (op, still) in ITS_OWN {
        sb.line(&format!("(deny {op})"));
        if !still.is_empty() {
            sb.line(&format!("(allow {op} {still})"));
        }
        if let Some(own) = own {
            sb.line(&format!("(allow {op} {own})"));
        }
    }
    let exec = super::super::profile::SANDBOX_EXEC;
    sb.line(&format!("(deny process-exec* (literal \"{exec}\"))")); // the same profile again would make it the starter below
    sb.line(&format!(
        "(deny process-exec* (require-all (literal {program}) (require-not (require-any (process-path {program}) (process-path \"{exec}\")))))"
    ));
}
