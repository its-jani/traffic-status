# Fingerprints

Every site you build with **scroll-craft** gets one row here, appended after it
ships. The registry exists so your next build can prove it is a different page
rather than a re-skin of one you already made.

This file is **yours**. It starts empty on purpose: the gate is about not
repeating *yourself*, so it has nothing to say until you have built something.

The rules and the gate live in the skill's
`references/uniqueness.md`. Short version:

**A new build must differ from EVERY row below on at least 4 of the 6
dimensions.** Four against each row individually, not four on average across the
table. If a planned build fails, change the plan. Never edit a row to make room
for it.

The six dimensions are: **grammar**, **nav treatment**, **hero device**,
**act-sequence shape**, **close pattern**, **signature move**.

Dimension 6 is free, because a signature move is unique by definition. So the
gate really asks for three more out of the remaining five, and a build that
changes only grammar and world will fail it.

---

## The registry

| Build | Grammar | Nav treatment | Hero device | Act-sequence shape | Close pattern | Signature move | World | Port |
|---|---|---|---|---|---|---|---|---|
| scroll-craft landing (`builds/scroll-craft`, 2026-10-30) | Typographic poster | None at all: one mono meta line at the top of the hero, which leaves with it. No bar, no wordmark, no anchor links | `pin` 2.2 with a greet-form subhead; the poster type tightens 5.5% across the pin and nothing else moves | 8 acts: pin > flow > flow > pin > pan > flow > flow > pin, 13.9 viewport-heights, no `scrub` anywhere | `pin` 1.2 that inverts the ground to paper, holds a single-value cue, and ends on a mono footer line | **The self-measure**: the peak pins and draws a spec sheet over its own type. ch measure, line count, tracking in em and WCAG contrast are read off the live layout by `audit.js` and annotated with accent hairlines driven by `--sc-p` | none | static HTML/CSS/JS |

One build. Every row after this one must differ from it on at least 4 of the
6 dimensions, measured against this row alone.

---

## What is taken

Add a bullet here whenever a build claims something a later build should avoid
reusing: a grammar, a nav treatment, a close pattern, a signature move, an
act-count-and-length band. The shared columns are what the next build inherits
as a constraint, so writing them down is the whole point.

Nothing is taken yet.

- **Typographic poster** grammar, built with no imagery at all: type is the ground.
- **No nav of any kind.** A single meta line inside the hero, which scrolls away with it.
- **A pinned close that inverts the ground** (dark page, paper resolve) and holds on one cue. No media, no CTA button, mono footer.
- **The self-measure**: live typography metrics printed over the page as annotation hairlines, then wiped away. Do not reuse this shape as "annotate a diagram".
- 8 acts at 13.9 viewport-heights, four of them pinned or panning.


---

## Appending a row

After shipping, add one line to the table and one bullet to **What is taken** if
the build claimed something new. Fill every column. Say what the build shares
with existing rows.

Rows are append-only. A build that has been superseded stays in the table,
because the space it occupies is still occupied.

---

## Worked example

The skill's author kept a registry of twelve builds across eight page grammars.
If you want to see what a filled-in table looks like, and which shapes tend to
collide, read `EXAMPLES.md` in the scroll-craft repository. Treat it as
illustration only: those rows are somebody else's builds and they do **not**
constrain yours.
