# Marketplace plan

Goal: people publish and share bots, you review each one first, buyers add them to their OpenBot, there are
reviews and add counts, creators can sell, and the site has analytics. This is a recommendation, not a survey.
Facts are from main at 3bc4f719 and the pages of x.ai (2026-11-24).

## What x.ai's marketplace shows (the model for structure)

- Index: title "Marketplace", a row of category tabs (All, From the team, Engineering, Sales, Marketing, Design,
  Personal, Recruiting, Product, Operations), cards with name, "by creator", one-line description and an Add
  button. No search, no sorting, no pagination.
- Bot page: avatar, name, "By creator", description, one Import button. Then four lists: Memories (facts it
  already knows), Skills (playbooks), Routines (jobs on their own), Integrations (apps it uses).
- What it does not show: rating, reviews, how many fleets added it, price, version, permissions, remix. Those
  are the owner's additions and are where OpenBot can be better than the model.

## What OpenBot supports today, by what blocks a launch

1. **No bot package.** There is no export and no import. A bot is a card in config (`[roles.bot-<id>]`, written
   by `bots/fragment.rs`), a routine list, memory and skills in `~/.tirith/manager/bot-<id>/.agents/skills`.
   Templates (`factory/bots/<id>/bot.toml` and `SKILL.md`, read by `bots/template.rs`) hold one routine and one
   skill and drop description, instructions and integrations. Needed: a package format (below).
2. **No owner-only import route.** `POST /api/bots/from-template` (`bots/from_template.rs`) only makes a bot from
   a template that ships with the install and is not behind `owner_only`. Other routes that are
   (`connector_page/routes.rs`, `secrets/api.rs`, `access/routes.rs`) show the pattern. An import route must be
   owner-only and must show what it will install before it does.
3. **Limits of a bot's own are partial.** A bot's connectors can be narrowed per bot (`bots/connect.rs`,
   `bots/grants.rs`: tools, deny, approval globs, written under `[roles.bot-<id>.connectors.*]`). Files are limited
   by `bots/folder.rs::writable` (own folder and the shared skills). Network, Safari tabs and other bots' skills
   have no per-bot limit (read from code, not run: sandboxes are not tested). Needed before a stranger's bot runs:
   a declared permission list in the package, shown at import, and enforced where it already can be.
4. **No review queue, no listings, no accounts, no reviews, no counts, no payments, no analytics.** None of it
   exists in this repo. agentsmessenger (`~/DEVELOP/agentsmessenger`) has rooms, signed messages, a notary, MCP and
   a deploy to the Mac mini (`tools/deploy/push.sh`), but no database, accounts or payments: its only
   dependencies are tweetnacl, ed2curve and qrcode.
5. **Identity exists.** `rooms/sealed/identity.rs` gives each OpenBot an ed25519 key. That is enough to sign a
   review, an install receipt or a publish request, with no account system.
6. The site is static (`site/`, GitHub Pages). The static marketplace on this branch lists only the official
   templates and shows no rating, count or price, because none can exist yet.

## Architecture

- **Package** (`.openbot-bot`, a zip or a folder): `bot.toml` (name, description, version, face, category,
  creator key, permissions it asks for), `routines.toml`, `skills/<name>/SKILL.md`, `instructions.md`,
  `integrations` (names only, never secrets, webhook addresses, memory or chats). Content hash plus creator
  signature (ed25519). Templates become packages without change of meaning.
- **Service** ("the Market"): a small Node or Rust service on the owner's Mac mini beside agentsmessenger, with
  SQLite, behind the same nginx and deploy script. Reasons: the owner already runs and deploys it, no vendor,
  and the identity (ed25519) is shared. Better than hosting on agentsmessenger itself: that repo is a notary with
  no state; a separate service keeps failures apart.
- **Publish**: the creator's OpenBot signs the package and posts it. The Market stores it as `pending`.
- **Review queue**: the owner gets a card in their own OpenBot chat (Approve / Reject with a reason, works from
  the phone), showing the permissions and the diff against the last approved version. Approval signs the
  listing with the owner's key. Nothing is public before that. Every update is reviewed again.
- **Listings**: the Market serves `listings.json` and package files. The static site reads it at build time (or
  the browser fetches it) so GitHub Pages stays the front end and the pages stay fast.
- **Add counts**: an OpenBot that imports a bot posts a signed install receipt (its key, bot hash). Count
  distinct keys, so one controller counts once. A count appears on the site only when real.
- **Reviews**: only a key that installed the bot may review; one review per key per version; signed. Shown with
  the count. The owner can hide abusive ones.
- **Selling**: the package is stored encrypted for paid bots; a purchase returns a licence signed for the buyer's
  key, and the import route checks it. Payments through Stripe Connect (creator payout, platform fee). Without
  payments the Market only lists free bots.
- **Analytics**: Plausible or a self-hosted equivalent (no cookies, no banner) on the site; the Market counts
  installs itself.
- **Where the data lives**: the Mac mini only (SQLite plus package files, backed up), nothing in the repo.

## Phases

1. **Static (this branch)**: official templates listed from one JSON file. Done.
2. **Package and import**: format, export of an owner's own bot, owner-only import with a preview of what it
   installs and asks. Works with a link or file, no server. Needs: nothing from the owner.
3. **Market service, free bots**: publish, review card, listings, install receipts, counts, reviews, analytics.
   Needs: the Mac mini service and domain, who may publish (invite or open), how people prove who they are
   (their OpenBot key, or also an email).
4. **Stranger's bot gates**: declared permissions enforced per bot (network, files, browser), automated checks
   before your review (secret scan, forbidden instructions). Needs: the owner's rule on what is refused.
5. **Paid bots**: Stripe Connect, licences, refunds, tax and terms. Needs: the owner's business and Stripe
   account, fee, and legal terms for creators and buyers.

## Ideas beyond the ask

- Show permissions and "asks first" on every listing; x.ai does not.
- Verified-by-you badge, plus versions and a changelog per bot.
- "Try it" with a read-only first run and no connectors.
- Remix: import a bot, change it, publish with credit to the original.
- Collections ("from the team", "most added this month") from the same JSON.
