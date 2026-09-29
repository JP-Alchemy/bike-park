# Bike Park — Game Design Document

29 Sept 2026 · JP Bothma

> This is the design document the project started from, kept in the repo as the source of
> truth. Two diagrams from the original (the core loop and the four-phase roadmap) are not
> included in this copy.

## Concept

Bike Park (working title) is an online multiplayer bike game for the browser: drop into a
live park with up to 12 riders, pull wheelies and tricks, crash spectacularly, and play
short rotating rounds together.

- **Audience:** kids and teens aged roughly 10–16 on CrazyGames, Poki and similar portals,
  mostly in the US, UK, Australia and Europe, often playing at school or with friends after
  school.
- **Pitch:** the bikes they ride or want to ride (electric dirt bikes, fatbikes, BMX),
  played online with friends in one click.
- **Why it can win:** the portals have 30+ bike games, but the popular ones are
  single-player level runners and the multiplayer ones are local split-screen. The
  long-running portal hits are online multiplayer hangout games. This combines a proven
  theme with the format that keeps players.
- **What it is not:** not another level-based stunt runner, not a realistic simulator, not
  pay-to-win, and not built on any real brand, rider or existing game's IP.

The design goal is simple: fun within 10 seconds, a reason to come back tomorrow, and a
reason to bring a friend.

## Format: 2.5D side view

The game uses 3D-looking graphics with 2D physics on a single side-view plane, the same
split Trials uses.

- Controls stay simple: throttle, brake and lean work on keyboard, touchscreen and school
  Chromebooks.
- Physics is cheap and predictable: easier to tune, runs on weak laptops, and much easier
  to sync between players.
- Everyone is visible at once: crashes and tricks read instantly, which helps spectating
  and short video clips.
- A track editor stays simple enough for a 12-year-old to use.
- Small downloads and cheaper art, which the portals' launch metrics reward.

The trade-off is a weaker free-roam feel than full 3D. Races use non-colliding "ghost"
riders; party modes switch collisions on; a shared side-view park between rounds provides
the hangout. A 3D free-roam mode is a possible later expansion, not part of this plan.

## Core loop

Every session is a chain of short rounds that always pay out, with unlocks that players
want to show friends.

The inner loop keeps a session going; the outer loop turns players into recruiters. Daily
challenges and events give a reason to restart the loop tomorrow.

## Vehicles and riding

The riding feel is the whole game: if the first 10 seconds aren't satisfying, nothing else
matters.

### Bike classes

Each class must feel clearly different, so players have favourites and argue about them.
All designs are generic, with no real brands.

| Class | Feel | Audience hook | Version |
| --- | --- | --- | --- |
| Electric dirt bike | Punchy torque, light, quiet, easy wheelies | Teen favourite in US, UK and Australia | 1 |
| Fatbike | Heavy, grippy, stable, slow to flip | Huge with Dutch and European teens | 1 |
| BMX | Very light, fast rotation, fragile landings | Trick and freestyle players | Later |
| Scooter | Short wheelbase, twitchy, spin tricks | Younger players | Later |
| Pit bike | Small, bouncy, cheap early unlock | New players | Later |

### Controls

- Throttle, brake, lean forward and lean back. One trick button with direction for named
  tricks.
- Keyboard, touch and gamepad from day one; the game must be fully playable with only a
  Chromebook keyboard.

### Wheelies

- A wheelie meter on every bike: points for distance, with a style bonus for balance near
  the tipping point.
- Wheelies are a core skill, not a gimmick: easy to start, hard to hold, satisfying to show
  off.

### Tricks and combos

- Named tricks such as flips, no-hander, superman and tail-whip, shown on screen when
  landed.
- Tricks chain into combos with a multiplier; a crash loses the unbanked combo.
- This gives good players months of mastery and gives kids shared names to talk about.

### Crashes

- Ragdoll rider physics with exaggerated but readable crashes.
- Automatic slow-motion replay of big crashes and big combos, followed by an instant
  respawn so crashes never feel punishing.

## Game modes

Rounds last 2–3 minutes and rotate automatically, so every session has variety without
menus.

| Mode | How it works | Collisions | Version |
| --- | --- | --- | --- |
| Race | First to the finish on a short track | Off (ghost riders) | 1 |
| Wheelie battle | Longest wheelie wins, or last rider still up | Off | 1 |
| Stunt battle | Highest trick and combo score in the time limit | Off | 1 |
| King of the hill | Hold the top zone longest | On | Later |
| Tag | One rider is "it" and passes it on by contact | On | Later |
| Group ride | Follow the leader and copy their tricks | Off | Later |
| Last rider standing | Survive a shrinking, hazard-filled track | On | Later |

Between rounds, all riders free-ride in a shared park hub: ramps, a wheelie strip and a spot
to show off cosmetics. This is where the hangout feeling lives.

## Retention and social systems

Players stay for progress and for friends; every system below serves one of those.

| System | What it does | Why it matters | Version |
| --- | --- | --- | --- |
| Daily and weekly challenges | "Hold a 10-second wheelie", "win a stunt battle" | Main lever for day-1 and day-7 retention | 1 |
| Gentle daily reward | Small reward for logging in; missing a day never takes anything away | Habit without resentment | 1 |
| Private lobbies with invite links | Friends play together from a shared link | How school-yard spread happens | 1 |
| Invite rewards | A cosmetic when a friend joins through your link | Free growth | 1 |
| Clip saving | One tap saves the last 10 seconds as a video with the game's name | Players market the game on TikTok and YouTube | 1 |
| Event system | Switch on limited-time maps, modes and cosmetics without a new release | Lets us ride trends in days, not weeks | 1 (skeleton) |
| Titles | Earned names such as "Wheelie King" shown above the rider | Status | Later |
| Crews | Small clans with a tag, colours and a weekly ranking | Belonging and rivalry | Later |
| Seasons | 6–8 week seasons with a free reward track and a new map theme | Long-term goals | Later |
| Track editor | Build and share tracks; the best rotate into official lobbies | Endless content and creator status | Later |
| Photo mode and garage | Show off your bike collection | Self-expression | Later |

## Customisation and monetization

The game earns from ads at natural pauses and from cosmetics; nothing for sale ever affects
who wins.

### Customisation

- Bike: paint, wraps, stickers, rims, LED lights, horn and exhaust sounds.
- Rider: outfits, helmets, on-bike emotes and dances.
- Effects: crash effects and trails.
- Everything is earnable with in-game coins; purchases speed this up or unlock exclusive
  looks.

### Ads

- A short ad after a round ends or after a respawn, never mid-round.
- Optional rewarded ads: double the round's coins, or try a locked bike for one round.
- Served through each portal's SDK; on our own domain through a web ad network.

### Purchases

- Cosmetic packs and, later, a season pass, sold at a visible price through the portal's
  payment system.
- Expect about 0.5–1% of players to buy something; this is where most revenue per player
  comes from once retention is strong.

### What we avoid

- Paid loot boxes: banned in Belgium, where CrazyGames is based, and a regulatory risk
  across Europe.
- Pay-to-win: kills multiplayer games.
- Punishing streaks and pressure timers aimed at children.
- Real brands, riders and other games' IP.

## Tech approach

One Rust physics core runs in both the browser (as WebAssembly) and the game server, so
riding behaves identically on both sides. These are proposals to confirm during the
prototype.

| Part | Proposal | Reason |
| --- | --- | --- |
| Physics | Rust with a 2D physics engine (for example Rapier 2D), compiled to WebAssembly for the client | Same code on client and server; fast on weak laptops |
| Rendering | Three.js with low-poly 3D models on the 2D plane | Small, well-supported, good on Chromebooks |
| Game server | Rust, authoritative, one process hosting many 12-player rooms | Cheap to run; stops most cheating |
| Networking | WebSockets at first; client-side prediction for your own bike, smoothing for others | Your bike feels instant; others look smooth |
| Lobbies | Small matchmaking service that fills rooms and adds bots when rooms are quiet | No one ever rides alone |
| Accounts and saves | Portal login and cloud save through each portal's SDK; guest play with no login | One click to play |
| Economy | Coins, unlocks and purchases validated on the server | Stops coin cheating |
| Analytics | Events for session start, first round, round end, day-1 and day-7 return | These are the metrics the portals judge us on |
| Hosting | US and EU regions at launch | Where the valuable audience is |

### Performance budgets

- First download under 10 MB; playable within 5 seconds on a school Chromebook.
- 60 frames per second on mid-range laptops, never below 30 on low-end ones.
- Server tick of about 30 per second; playable up to roughly 150 ms of lag.

## Safety and compliance

The audience is young, so safety is designed in from day one rather than added later.

- **No free-text chat.** Players communicate through preset quick-chat phrases and emotes
  only.
- **Filtered names.** Player names, crew names and track names pass a word filter; bad names
  are reset to a generated one.
- **Report and block** on every player, with a simple review queue.
- **Minimal data.** Guest play needs no personal data; portal logins provide only what the
  portal shares. A privacy policy and age handling that meet EU children's data rules and
  the US children's privacy law (COPPA), with a lawyer's review before launch.
- **Portal rules.** CrazyGames requires content suitable for age 12 and up (PEGI 12); the
  game should comfortably meet that.
- **Riding stays in parks and tracks.** No traffic dodging or street riding among cars, to
  avoid glorifying dangerous riding.
- **Original IP only.** All bikes, riders, names, art and sounds are our own or properly
  licensed.

## Scope and roadmap

The project moves in four phases, and each gate must pass before the next phase spends real
time or money.

A failed gate means fix and retest, or stop; the prototype gate is the cheapest place to
learn the idea doesn't work.

## Launch plan and success metrics

Launch on our own site first to prove the netcode, then on CrazyGames and Poki, then grow
through clips and creators rather than paid ads.

1. Own .io domain: test stability and gather first metrics with real players.
2. CrazyGames: apply for direct Full Launch as a multiplayer game; multiplayer titles can
   skip the two-week Basic Launch.
3. Poki and other portals in the same window; no exclusivity unless a portal offers a deal
   worth it.
4. Launch week: about €500–2,000 in small ads and creator videos to fill lobbies and learn
   which clips spread.
5. Updates every 1–2 weeks for the first three months; the portals' algorithms reward
   improving numbers.
6. Mobile app versions only after the web version proves itself.

### Targets

These are our own targets; only the session length comes from CrazyGames' guidance
(successful titles often see 10+ minutes).

| Metric | Target |
| --- | --- |
| Visitors who start playing | 80% or more |
| Average session length | 10 minutes or more |
| Players returning the next day | 25% or more |
| Players returning after a week | 8% or more |
| Players who invite a friend | 5% or more |
| Players who buy something | 0.5% or more |

## Budget and business case

Maximum cash at risk before real player data is about €3,000–6,000; the larger cost is
roughly 400–600 hours of building time. All figures are rough estimates, possibly off by 2x
either way.

### Cash to build and launch

| Item | Lean | Comfortable |
| --- | --- | --- |
| Art: bikes, riders, maps, first cosmetics | €1,000 | €5,000–10,000 |
| Sound and music | €300 | €1,500 |
| Privacy and legal | €300 | €1,500 |
| Servers during development | €150 | €300 |
| Accounts, domain, tools | €150 | €300 |
| Playtesting | €100 | €500 |
| Launch ads and creators | €500 | €2,000 |
| **Total** | **~€2,500** | **~€11,000–16,000** |

### Monthly outcomes

After the portals' share (about 40% of ad revenue per the CrazyGames jam terms), servers
included.

| Scenario | Daily players | Revenue | Profit |
| --- | --- | --- | --- |
| Flop | under 500 | under €100 | about zero |
| Modest | 2,000 | ~€800 | ~€700 |
| Solid hit | 20,000 | ~€8,500 | ~€8,000 |
| Top hit | 200,000+ | ~€80,000+ | ~€75,000+ |

Most games land in the first two rows. The plan therefore keeps cash spend low until the
prototype and launch metrics justify more. Revenue would run through Prymer BV; VAT handling
of the portals' self-billing invoices to be checked with the accountant.

## Risks and open questions

The biggest risk is the riding feel; the prototype exists to settle it before anything else
is built.

| Risk | Mitigation |
| --- | --- |
| Riding feels floaty or unfair | Prototype nothing but the riding first; test with kids before building more |
| Looks like "just another bike game" on a crowded shelf | Thumbnail and first 10 seconds must show online play with friends |
| Empty lobbies at launch | Bots fill quiet rooms; launch-week creator push |
| Established studios copy online multiplayer | Move fast, build community, keep content flowing |
| Server costs grow with success | Efficient netcode, small rooms, costs scale behind revenue |
| Child safety incident | No free chat, filtered names, report and block from day one |
| Build time next to a full-time job | Strict version-1 scope; prototype gate before committing months |

### Open questions

- [ ] Final game name and a check that the name and domain are free
- [ ] Confirm no strong online multiplayer bike game already exists on the major portals
- [ ] Rapier 2D and Three.js versus alternatives, decided during the prototype
- [ ] Where to find 20 or so kids aged 10–16 for playtests
- [ ] Art direction: stylised low-poly or cleaner cartoon look
- [ ] Freelancers for art and sound, and when to bring them in
