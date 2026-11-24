//! The options of the guide's commands.

pub const USE: &str =
    "neobot setup [--server] [--boot] [--port <port>] [--agents <a,b>] [--primary <agent>] [--yes]
neobot connect [<agent>] [--token] [--yes]
neobot service install [--boot] | status | start | stop | restart | uninstall | logs
neobot doctor";

#[derive(Default, Debug)]
pub struct Opts {
    /// The words that are no option.
    pub words: Vec<String>,
    /// Install the service.
    pub server: bool,
    /// The service starts with nobody logged in.
    pub boot: bool,
    /// Ask nothing: defaults.
    pub yes: bool,
    /// The agent's token comes on stdin.
    pub token: bool,
    pub port: Option<u16>,
    pub agents: Option<Vec<String>>,
    pub primary: Option<String>,
}

pub(super) fn parse(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value.to_string())),
            _ => (arg.as_str(), None),
        };
        let mut value = || {
            inline
                .clone()
                .or_else(|| rest.next().cloned())
                .ok_or(format!("{name} needs a value"))
        };
        match name {
            "--server" => o.server = true,
            "--boot" => o.boot = true,
            "--yes" | "-y" => o.yes = true,
            "--token" => o.token = true,
            "--port" => {
                let port = value()?.parse::<u16>().ok().filter(|p| *p >= 1024);
                o.port = Some(port.ok_or("--port is a number from 1024 to 65535")?);
            }
            "--agents" => {
                let listed = value()?;
                let names = listed.split(',').map(str::trim).filter(|a| !a.is_empty());
                o.agents = Some(names.map(String::from).collect());
            }
            "--primary" => o.primary = Some(value()?),
            _ if name.starts_with('-') => return Err(format!("no option {name}\n{USE}")),
            _ => o.words.push(arg.clone()),
        }
    }
    Ok(o)
}
