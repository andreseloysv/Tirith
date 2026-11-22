# Tirith · Enseñar

A Manifest V3 extension for your Edge: you do a task once, it records the steps, and your bot writes a draft skill
from them (docs/BOTS.md, part 5).

## Install it in two steps

1. In Tirith open your bot, tab **Enseñar**, press **Download the extension** (the page serves it, so it works from
   a server reached through an SSH tunnel too), unzip it, and in Edge open `edge://extensions`, turn on **Developer
   mode** and press **Load unpacked** («Cargar descomprimida»): choose the unzipped folder. (From a checkout, choose
   `tools/teach` itself.)
2. Copy the link the tab shows and paste it once in the extension (its button in the toolbar, «Enlace del bot»,
   **Guardar**).

Then press **Grabar** (in the extension or in the tab «Enseñar»), do the task, and press **Parar**.

## What it records

- Pages: their URL (secret query and fragment values, and the password of `user:password@`, masked) and title.
- Clicks: the element's text, its role and a short selector (never what is typed in a contenteditable).
- Typing: the field's label and value, when the value is committed.
- The time of each step. What you write in the bot's chat meanwhile goes with the step it follows. Your clock and
  the daemon's may differ: the daemon says how long is left and times each step on its own clock.

Passwords (also one shown as text), one-time codes and cards are masked as «•••»: `type=password`, `autocomplete`
`one-time-code`, `cc-*` or holding `password`, and fields whose name or label has a whole word such as pwd, pass,
passwd, passcode, password, contraseña, clave, secret, token, otp, pin, cvv, tarjeta, card, código, or «verification
code» / «security code». The daemon masks them again. It records only while recording, and stops by itself after 10
minutes.

## Rehearsals

After writing the draft, your bot can rehearse it here: the extension opens the pages, clicks and types in a tab of
its own, under the notice «Tirith está ensayando», and sends Tirith a screenshot after each step. A click on a control
whose words send, buy, pay, delete or publish (English or Spanish: `outward.js`), or that the skill lists under what
needs your yes, is refused, here by the element it found and in the daemon by what the bot named; the bot is told to
stop there. The rehearsal stops by itself after 10 minutes. In the tab «Enseñar» you see the screenshots in order and
press **Aprobar** (only after a finished rehearsal with a step that worked; the draft then moves into the bot's skills)
or **Crear rutina**.

The daemon takes the steps only with the bot's token, and answers CORS only to this extension's origin
(`chrome-extension://bogleilobfephnbnjibgoofoeipkhfef`, fixed by the `key` in manifest.json).

`words.js` and `outward.js` hold no page code: `node --test scripts/checks/teach-words.test.mjs` runs them against
the table the daemon passes too (`rust/tirith/checks/teach_table.json`).
