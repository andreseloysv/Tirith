---
name: {name}
description: When to use it, in one sentence.
---

## Cuándo usarla
What request or routine sets it going, and when it does not apply.

## Entradas y accesos
What it needs to start: request data, accounts, pages, files and permissions.

## Pasos
The numbered steps, in order, with where to look, what to filter and what to skip. When the steps are commands that already worked together, end with them as one shell call (joined with &&, their checks in it), so the next run is one call and not one per step.

## Cómo validar
How to check that the result is right before giving it: each check on one line.

## Qué devuelve
The format and length of the answer (for example: three lines, one per new offer, or «∅» if it looked and nothing is new; never «∅» if it could not).

## Qué requiere aprobación
What is not done without the owner's yes: sending, buying, deleting, publishing or touching production.

## Si algo falla
What to do when an access is missing or a step no longer works, and what to tell the owner in one line that starts with «✗».
