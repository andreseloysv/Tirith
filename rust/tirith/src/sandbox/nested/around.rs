//! What the daemon adds to a request and takes from no caller, and the request as the kernel names its paths.
use super::{doors::Doors, limits::Around};
use super::{place, sb::real, Request};
use crate::app::Shared;

pub(super) fn around(
    app: &Shared,
    mut r: Request,
    doors: Doors,
) -> Result<(Request, Around), String> {
    if !r.program.starts_with('/') || !r.cwd.starts_with('/') {
        return Err("a program and a folder by their whole paths".into()); // a program named like an option would be one
    }
    let partly = |p: &String| !p.is_empty() && !p.starts_with('/');
    if r.roots.iter().any(partly) || partly(&r.state) || partly(&r.out) {
        return Err("folders by their whole paths".into());
    }
    r.program = real(&r.program); // the file the profile's rules name
    let mut kept = vec![app.home.join("net"), app.home.join("config.toml")];
    kept.extend(place::socket(&app.home));
    kept.extend(crate::app::exe());
    doors.told(&mut r);
    let around = Around {
        doors,
        kept: kept.iter().map(|p| p.display().to_string()).collect(),
        hidden: crate::app::kept(&app.home).to_vec(),
    };
    Ok((r, around))
}
