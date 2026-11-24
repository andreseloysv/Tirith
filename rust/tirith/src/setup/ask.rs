//! Asking the owner on the terminal. `curl … | sh` leaves stdin to the pipe, so the terminal is /dev/tty; with --yes, or without one, every question takes its default. On the page there is no terminal: the owner's tap was the yes.
use std::fs::File;
use std::io::{BufRead, BufReader, IsTerminal, Write};
use std::process::Stdio;

pub struct Ask {
    tty: Option<File>,
    /// --yes: every question takes its default.
    yes: bool,
    /// The agent whose setup the page runs: what programs print goes to it.
    page: Option<String>,
}

impl Ask {
    pub fn new(yes: bool) -> Ask {
        let open = || {
            let mut file = File::options();
            file.read(true).write(true).open("/dev/tty").ok()
        };
        let tty = open().filter(|f| f.is_terminal());
        Ask {
            tty,
            yes,
            page: None,
        }
    }

    /// For the page's setup of `alias`: every question takes its default.
    pub fn page(alias: &str) -> Ask {
        Ask {
            tty: None,
            yes: true,
            page: Some(alias.to_string()),
        }
    }

    /// The agent whose setup the page runs, when it is the page that asks.
    pub fn on_page(&self) -> Option<&str> {
        self.page.as_deref()
    }

    /// Whether a person can answer.
    pub fn person(&self) -> bool {
        self.tty.is_some() && !self.yes
    }

    /// The terminal as a child's stdin, for what asks by itself (a login, sudo); nothing without one.
    pub fn stdin(&self) -> Stdio {
        self.tty
            .as_ref()
            .filter(|_| !self.yes)
            .and_then(|f| f.try_clone().ok())
            .map_or_else(Stdio::null, Stdio::from)
    }

    /// A question's answer, trimmed; `default` on Enter, with --yes or with no terminal.
    pub fn text(&self, question: &str, default: &str) -> String {
        let Some(mut tty) = self.tty.as_ref().filter(|_| !self.yes) else {
            return default.to_string();
        };
        let shown = match default {
            "" => format!("{question}: "),
            d => format!("{question} [{d}]: "),
        };
        let _ = tty.write_all(shown.as_bytes()).and_then(|_| tty.flush());
        let mut line = String::new();
        match BufReader::new(tty).read_line(&mut line) {
            Ok(n) if n > 0 && !line.trim().is_empty() => line.trim().to_string(),
            _ => default.to_string(),
        }
    }

    pub fn yes_no(&self, question: &str, default: bool) -> bool {
        let Some(()) = self.person().then_some(()) else {
            return default;
        };
        let hint = if default { "Y/n" } else { "y/N" };
        match self
            .text(&format!("{question} ({hint})"), "")
            .to_lowercase()
            .as_str()
        {
            "" => default,
            said => matches!(said, "y" | "yes" | "s" | "si" | "sí"),
        }
    }
}
