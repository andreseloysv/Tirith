//! What a rehearsal never does by itself: a click on a control whose words send, buy, pay, delete or publish (English and Spanish; what the skill lists as needing the owner's yes is in approvals.rs). tools/teach/outward.js reads the same words, and both pass checks/teach_table.json.
use super::names::{fold, words};
/// Words of a control that send, buy, pay, delete, publish or confirm, as written without accents.
const SENDS: [&str; 56] = [
    "send",
    "sends",
    "sending",
    "submit",
    "buy",
    "purchase",
    "pay",
    "checkout",
    "order",
    "delete",
    "remove",
    "publish",
    "post",
    "reply",
    "forward",
    "share",
    "archive",
    "confirm",
    "book",
    "subscribe",
    "donate",
    "transfer",
    "trash",
    "enviar",
    "envia",
    "envie",
    "enviad",
    "comprar",
    "compra",
    "compre",
    "pagar",
    "paga",
    "pague",
    "pedir",
    "pedido",
    "borrar",
    "borra",
    "eliminar",
    "elimina",
    "quitar",
    "quita",
    "publicar",
    "publica",
    "publique",
    "responder",
    "responde",
    "reenviar",
    "reenvia",
    "compartir",
    "comparte",
    "archivar",
    "confirmar",
    "confirma",
    "reservar",
    "reserva",
    "papelera",
];
/// What the bot is told when a step is refused.
pub(super) const STOP: &str = "Do not run this step or any other that does the same, not even to see what happens: end with `neobot teach stop` and say in the chat in one line where you stopped (as «Paré antes de 4. Click «Enviar»: requiere tu aprobación»).";
/// Whether `text` (a control's words, its aria-label, title, value, or the selector a bot named) says it sends, buys, pays, deletes or publishes.
pub fn sends(text: &str) -> bool {
    words(&fold(text))
        .iter()
        .any(|w| SENDS.contains(&w.as_str()))
}
