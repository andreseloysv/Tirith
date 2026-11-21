# Tirith · Enseñar

A Manifest V3 extension for your Edge: you do a task once, it records the steps, and your bot writes a draft skill
from them (docs/BOTS.md, part 5).

## Install it in two steps

1. In Edge open `edge://extensions`, turn on **Developer mode** and press **Load unpacked** («Cargar
   descomprimida»): choose this folder, `tools/teach`.
2. In Tirith open your bot, tab **Enseñar**, copy its link and paste it once in the extension (its button in the
   toolbar, «Enlace del bot», **Guardar**).

Then press **Grabar** (in the extension or in the tab «Enseñar»), do the task, and press **Parar**.

## What it records

- Pages: their URL and title.
- Clicks: the element's text, its role and a short selector.
- Typing: the field's label and value, when the value is committed.
- The time of each step. What you write in the bot's chat meanwhile goes with the step it follows.

Passwords, one-time codes and cards are masked as «•••» (`type=password`, `autocomplete` `one-time-code` or `cc-*`,
and fields whose name or label looks like card, cvv, otp, pin or código); the daemon masks them again. It records
only while recording, and stops by itself after 10 minutes.

## Rehearsals

After writing the draft, your bot can rehearse it here: the extension opens the pages, clicks and types in a tab of
its own, under the notice «Tirith está ensayando», and sends Tirith a screenshot after each step. The bot stops
before any step that needs your yes, and the rehearsal stops by itself after 10 minutes. In the tab «Enseñar» you see
the screenshots in order and press **Aprobar** (the bot uses the skill from then on) or **Crear rutina**.

The daemon takes the steps only with the bot's token, and answers CORS only to this extension's origin
(`chrome-extension://bogleilobfephnbnjibgoofoeipkhfef`, fixed by the `key` in manifest.json).
