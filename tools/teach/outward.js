// What a rehearsal never does by itself: a click on a control whose words send, buy, pay, delete or publish (English and
// Spanish). No page and no browser in it; the daemon refuses the same words (rust/tirith/src/teach/outward.rs) and both pass
// rust/tirith/checks/teach_table.json. Loaded before act.js in every page.
/* global teachWords */
/* exported teachSends, teachOutward */
var SENDS = ['send', 'sends', 'sending', 'submit', 'buy', 'purchase', 'pay', 'checkout', 'order', 'delete', 'remove', 'publish', 'post',
  'reply', 'forward', 'share', 'archive', 'confirm', 'book', 'subscribe', 'donate', 'transfer', 'trash', 'enviar', 'envia', 'envie', 'enviad',
  'comprar', 'compra', 'compre', 'pagar', 'paga', 'pague', 'pedir', 'pedido', 'borrar', 'borra', 'eliminar', 'elimina', 'quitar', 'quita',
  'publicar', 'publica', 'publique', 'responder', 'responde', 'reenviar', 'reenvia', 'compartir', 'comparte', 'archivar', 'confirmar',
  'confirma', 'reservar', 'reserva', 'papelera'];

/** Whether the text says it sends, buys, pays, deletes or publishes (accents do not matter: Envía is envia). */
function teachSends(text) {
  var plain = String(text || '').replace(/[áàä]/g, 'a').replace(/[éèë]/g, 'e').replace(/[íìï]/g, 'i').replace(/[óòö]/g, 'o')
    .replace(/[úùü]/g, 'u').replace(/ñ/g, 'n').replace(/[ÁÀÄ]/g, 'A').replace(/[ÉÈË]/g, 'E').replace(/[ÍÌÏ]/g, 'I')
    .replace(/[ÓÒÖ]/g, 'O').replace(/[ÚÙÜ]/g, 'U').replace(/Ñ/g, 'N');
  return teachWords(plain).some(function (w) { return SENDS.includes(w); });
}

/** The first of a control's words (its text, aria-label, title, value) that says it sends, or ''. */
function teachOutward(parts) {
  return parts.find(function (p) { return p && teachSends(p); }) || '';
}
if (typeof module !== 'undefined') module.exports = { teachSends, teachOutward };
