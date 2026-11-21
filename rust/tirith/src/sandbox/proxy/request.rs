//! A request's head: the first line, the headers and where it goes.

pub struct Request<'a> {
    pub method: &'a str,
    pub target: &'a str,
    pub version: &'a str,
    pub headers: Vec<&'a str>,
}

impl<'a> Request<'a> {
    pub fn parse(head: &'a str) -> Self {
        let mut lines = head.split("\r\n");
        let mut first = lines.next().unwrap_or("").split(' ');
        let (method, target, version) = (
            first.next().unwrap_or(""),
            first.next().unwrap_or(""),
            first.next().unwrap_or("HTTP/1.1"),
        );
        let headers = lines.filter(|l| !l.is_empty()).collect();
        Request {
            method,
            target,
            version,
            headers,
        }
    }

    pub fn connect(&self) -> bool {
        self.method.eq_ignore_ascii_case("CONNECT")
    }

    /// `host[:port]` and the path to send on (empty for `CONNECT`); `None` for a target that is not `http://`.
    pub fn destination(&self) -> Option<(&'a str, String)> {
        if self.connect() {
            return Some((self.target, String::new()));
        }
        let target = self.target;
        let rest = target
            .get(..7)
            .filter(|s| s.eq_ignore_ascii_case("http://"))
            .map(|_| &target[7..])?;
        let at = rest.find(['/', '?']).unwrap_or(rest.len());
        let path = &rest[at..];
        Some((
            &rest[..at],
            if path.starts_with('/') {
                path.to_string()
            } else {
                format!("/{path}")
            },
        ))
    }
}

/// `host[:port]` with an optional `[v6]` host.
pub fn host_port(text: &str, default: u16) -> Option<(String, u16)> {
    let (host, port) = match text.strip_prefix('[') {
        Some(v6) => {
            let (h, rest) = v6.split_once(']')?;
            (h, rest.strip_prefix(':'))
        }
        None => text
            .rsplit_once(':')
            .map_or((text, None), |(h, p)| (h, Some(p))),
    };
    let port = port.map_or(Some(default), |p| p.parse().ok())?;
    (!host.is_empty()).then(|| (host.trim_end_matches('.').to_lowercase(), port))
}
