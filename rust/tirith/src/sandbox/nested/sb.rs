//! A profile's text as it is written: paths travel as parameters, never as profile text, each as the kernel names it.
use std::collections::BTreeMap;
use std::path::Path;

pub(super) struct Sb {
    text: String,
    params: BTreeMap<String, String>,
}

impl Sb {
    /// `base`, whole, and what follows only takes away from it.
    pub(super) fn after(base: &str) -> Sb {
        Sb {
            text: base.to_string(),
            params: BTreeMap::new(),
        }
    }

    pub(super) fn line(&mut self, rule: &str) {
        self.text += rule;
        self.text += "\n";
    }

    /// `(param "Pn")`, for a path that is already the kernel's name.
    pub(super) fn path(&mut self, path: &str) -> String {
        let next = format!("P{}", self.params.len());
        let name = self.params.entry(path.to_string()).or_insert(next);
        format!("(param \"{name}\")")
    }

    pub(super) fn done(self) -> (String, Vec<(String, String)>) {
        let params = self.params.into_iter().map(|(path, name)| (name, path));
        (self.text, params.collect())
    }
}

/// `path` as the kernel names it (/tmp is /private/tmp), also when its end is not there yet.
pub(super) fn real(path: &str) -> String {
    let path = Path::new(path);
    let (mut head, mut tail) = (path, vec![]);
    loop {
        if let Ok(found) = head.canonicalize() {
            let whole = tail.iter().rev().fold(found, |p, part| p.join(part));
            return whole.display().to_string();
        }
        let (Some(parent), Some(last)) = (head.parent(), head.file_name()) else {
            return path.display().to_string();
        };
        tail.push(last.to_owned());
        head = parent;
    }
}

/// `path` with its last name as written: a link itself, not where it points.
pub(super) fn named(path: &str) -> String {
    let path = Path::new(path);
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(last)) => Path::new(&real(&parent.display().to_string()))
            .join(last)
            .display()
            .to_string(),
        _ => path.display().to_string(),
    }
}
