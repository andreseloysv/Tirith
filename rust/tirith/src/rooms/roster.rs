//! Each paired phone is told who else is in the room (`tirith-roster <ids>`) when that changed.
use super::{poll::Out, Room};
/// Sends `tirith-roster <ids>` to each paired phone when the set of phones and accepted managers changed.
pub(super) fn roster(r: &mut Room, out: &mut Out) {
    let mut ids: Vec<String> = r
        .devices
        .iter()
        .chain(r.managers.iter().filter(|m| m.accepted))
        .map(|p| p.key_id.clone())
        .collect();
    ids.sort();
    let sig = ids.join(",");
    let _ = (r.kind == "manager" && std::mem::replace(&mut r.told.roster, sig.clone()) != sig)
        .then(|| {
            r.devices.iter().for_each(|d| {
                out.send.push((
                    format!(
                        "tirith-roster {}",
                        ids.iter()
                            .filter(|i| **i != d.key_id)
                            .map(String::as_str)
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                    vec![d.public.clone()],
                ))
            })
        });
}
