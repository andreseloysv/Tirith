//! `neobot mcp --bot <id>`: the gateway a bot's run starts as its MCP server, on stdio. Its stdout is the protocol's alone; why it stops goes to stderr.
use super::{door::Door, stdio::serve};
use crate::client::Flag;

pub fn cli(pos: &[&str], flag: Flag) -> Option<Result<(), String>> {
    let ["mcp"] = pos else {
        return None;
    };
    Some(match flag("--bot").filter(|b| !b.is_empty()) {
        Some(bot) => serve(
            std::io::stdin().lock(),
            std::io::stdout().lock(),
            &Door::here(),
            &bot,
        ),
        None => {
            Err("usage: neobot mcp --bot <id> (a bot run's MCP server, started by NeoBot)".into())
        }
    })
}
