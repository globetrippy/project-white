# Project White — Terminal UI/UX Specification v1

> Secure. Minimal. Predictable.

---

## 1. UX Philosophy

Project White's terminal UI follows five axioms, in priority order:

**1. Confidence over excitement.**  
The user is transferring data they care about. Every visual cue should say "this worked" or "this will work" — never "wow look at this." Remove anything that feels like celebration. A single dim `✓ Installed` from Homebrew communicates more trust than a fireworks ASCII banner.

**2. One screen at a time.**  
The terminal never scrolls during a transfer. Each of the three layouts replaces the previous one entirely. The cursor stays at the bottom of the layout. The user never has to scroll up to find context they lost.

**3. Every visible value answers a question.**  
If a user would never ask "what's my current file?" or "how much time is left?", don't show it. Conversely, if they would ask "did the hash check pass?", show it clearly. Remove data that only a developer debugging the protocol would need — put that behind `--debug`.

**4. Layouts are containers; data is the content.**  
The same physical screen estate holds the same logical role across states. The verification area becomes the transfer area becomes the summary. The user's eyes learn where to look. Repainting the entire layout on state change (not scrolling) preserves that spatial memory.

**5. Typography and alignment are the UI.**  
No icons. No emoji. No color gradients. The only tools are: whitespace, alignment, weight (bold/dim), progress bars, and box-drawing characters. If it can't be expressed with those, it doesn't belong.

---

## 2. Information Hierarchy

```
Level 1 — Focal (single piece of data, visually dominant)
  Session code
  % complete
  Final status (Success / Failed / Interrupted)

Level 2 — Primary metrics (the data people need most)
  Transferred / total size
  Current file name
  Files completed
  Speed
  ETA

Level 3 — Secondary metrics (useful context)
  Original size, compressed size, savings
  Duration
  Average speed

Level 4 — Trust indicators (verification status)
  Fingerprint
  Connection type
  Integrity bar
  Hash verification bar

Level 5 — Instructional / status text
  "Waiting for receiver…"
  "Verify this fingerprint if using an untrusted network."
```

---

## 3. Layout Evolution

```
[Idle/init]     →     Layout 1 (Secure Session)
                      ┌─────────────────────────────┐
                      │          Session Code        │  ← focal
                      │     Expiration timer + bar   │
                      │     Status line              │
                      └─────────────────────────────┘

Receiver joins  →     Layout 2 (Device Verification)
                      ┌─────────────────────────────┐
                      │     Device info + fingerprint│
                      │     Explanation              │
                      │     Selection menu           │
                      └─────────────────────────────┘

User approves   →     Layout 2 (Live Transfer)         ← same outer box, content replaced
                      ┌─────────────────────────────┐
                      │     Progress bar + %         │
                      │     Size / speed / ETA       │
                      │     Current file             │
                      │     Files done               │
                      │     Compression info         │
                      │     Connection + integrity   │
                      └─────────────────────────────┘

Transfer done   →     Layout 3 (Transfer Complete)
                      ┌─────────────────────────────┐
                      │     Status (Success)         │
                      │     Size / compression       │
                      │     Files / folders / time   │
                      │     Hash + integrity checks  │
                      └─────────────────────────────┘
```

---

## 4. Complete Layout Specifications

### 4.1 Layout 1 — Secure Session

**Purpose:** Create a session code and wait for a peer to connect.  
**Trigger:** `pw send <dir>` or `pw receive`  
**States:** Preparation → Scanning → Session Ready (all in same box)

```
┌───────────────────────── Secure Session ─────────────────────────┐
│                                                                  │
│                         WHITE-8JKD-Q2L9                          │
│                                                                  │
│  Session expires in                                              │
│  ████████████████████████████████████████████░░░░░░░░  04:58     │
│                                                                  │
│  Waiting for receiver…                                           │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

**Spec:**

- Box: `┌─┐` / `│` / `└─┘` using regular box-drawing (U+2500–2573), no rounded corners
- Title: centered, padded with spaces, dim
- Session code: bold, centered, monospace, 14-char (`XXXX-XXXX-XXXX` format)
- Expiration bar: `█` filled, `░` unfilled, right-aligned with timer text
- Status: dim, centered, below bar
- When preparing/scanning: the box exists but shows "Preparing transfer…" or "Scanning 342 files…" in the status line. The session code area is dim/empty until the session is created. Once created, the code appears (bold) and the timer starts — the user sees the morph live but the box doesn't move.

**Transitions:**
- Preparation → Scanning: status text updates in place
- Scanning → Ready: code appears, timer begins

### 4.2 Layout 2a — Device Verification

**Purpose:** Verify and approve the connecting peer.  
**Trigger:** Receiver initiates connection to the session.

```
┌────────────────────── Device Verification ───────────────────────┐
│                                                                  │
│  Device            Windows-Workstation                           │
│  Platform          Windows 11                                    │
│                                                                  │
│  Fingerprint                                                     │
│                     91FA:C417:82DE:AD61:73F1                     │
│                                                                  │
│  Verify this fingerprint with the receiver if using              │
│  an untrusted network.                                           │
│                                                                  │
│                                                                  │
│  ❯ Yes, approve connection                                       │
│    No, reject connection                                         │
│    Abort session                                                 │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

**Spec:**

- Device info: left-aligned, label + value on same line, labels dimmed
- Fingerprint: bold, monospace, colon-separated hex pairs
- Explanation: dim, justified to box width
- Menu: `❯` as cursor, arrow keys navigate, Enter selects
- Selected item: bold
- Unselected items: dim
- The entire Layout 2 box replaces Layout 1 — same width, same position

### 4.3 Layout 2b — Live Transfer

**Purpose:** Show transfer progress in real time.  
**Trigger:** User approves at verification screen.

```
┌───────────────────────── File Transfer ──────────────────────────┐
│                                                                  │
│  ████████████████████████████████████░░░░░░░░░░░░  74%           │
│                                                                  │
│  4.82 GB / 6.51 GB                                               │
│                                                                  │
│  Current File                                                    │
│  src/network/session.rs                                          │
│                                                                  │
│  Files             248 / 391                                     │
│  Speed             842 MB/s                                      │
│  ETA               00:03                                         │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Original          6.51 GB                                       │
│  Compressed        5.43 GB                                       │
│  Saved             1.08 GB (16.6%)                               │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Connection        Direct P2P                                    │
│  Integrity         ████████████████████████████████████ Live     │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

**Spec (*six rows of content*):**

| Row | Content | Style |
|-----|---------|-------|
| 1 | Progress bar + percentage | `█` + bold right-aligned `NN%` |
| 2 | Transferred / Total | `NN.NN GB / NN.NN GB`, monospace |
| 3 | Current file (label+path) | label dim, path normal, truncated right if too long |
| 4 | Files / Speed / ETA | three columns, labels dim |
| 5 | Divider | `├─ ┤` |
| 6 | Compression info | three rows, label dim, values normal |
| 7 | Divider | `├─ ┤` |
| 8 | Connection type + Integrity bar | label dim, values normal, integrity bar is `█` only |

**Update behavior:**
- Every ~100ms or on data arrival: progress bar, percentage, size, speed, ETA update
- On file boundary: current file name and files count update
- Integrity bar: fills left-to-right as chunks arrive; when fully filled, label changes from `Live` to `Verified`
- No screen clearing — only overwrite the values that changed, using a single `\r` + inline escape sequence
- The layout never scrolls

**Compression info visibility:**
- If compression is disabled or savings < 1%, hide the compression section entirely (rows 5-6 removed)
- Show a single divider row connecting speed section to connection section

### 4.4 Layout 2b — Verification Mode (sender perspective)

When the user is the **sender** (not the receiver), Layout 2 shows:

```
┌───────────────────────── File Transfer ──────────────────────────┐
│                                                                  │
│  ████████████████████████████████████░░░░░░░░░░░░  74%           │
│                                                                  │
│  4.82 GB / 6.51 GB                                               │
│                                                                  │
│  Current File                                                    │
│  src/network/session.rs                                          │
│                                                                  │
│  Files             248 / 391                                     │
│  Speed             842 MB/s                                      │
│  ETA               00:03                                         │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Original          6.51 GB                                       │
│  Compressed        5.43 GB                                       │
│  Saved             1.08 GB (16.6%)                               │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Receiver         Windows-Workstation                            │
│  Integrity         ████████████████████████████████████ Live     │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

Only the bottom section differs. In the sender's view, the receiver's device name replaces "Connection Direct P2P". The integrity bar is identical.

### 4.5 Layout 3 — Transfer Complete

**Purpose:** Final report confirming the transfer succeeded or reporting failure.  
**Trigger:** All chunks verified and `Complete` packet acknowledged.

```
┌─────────────────────── Transfer Complete ────────────────────────┐
│                                                                  │
│  Status              Success                                     │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Original Size       6.51 GB                                     │
│  Transferred Size    5.43 GB                                     │
│  Compression Saved   1.08 GB (16.6%)                             │
│                                                                  │
│  Files               391                                         │
│  Folders             31                                          │
│                                                                  │
│  Duration            18.2 sec                                    │
│  Average Speed       842 MB/s                                    │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Hash Verification                                               │
│  ████████████████████████████████████████████  100%              │
│                                                                  │
│  Integrity Check                                                 │
│  ████████████████████████████████████████████  Verified          │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

**Spec:**
- Status: `Success` in bold (or `Failed` / `Interrupted` in bold, not red)
- Metrics: grouped by type, no dividers between sub-groups within a section
- Hash bar: fills to 100% on final verification pass
- Integrity: static `Verified` (already filled during transfer)
- The boxes `┌ ─ ┤ └ ┤` adapt to content — if compression section was hidden during transfer, it stays hidden in the summary

---

## 5. Error Screens

### 5.1 Connection Lost (Mid-Transfer)

```
┌──────────────────────── Connection Lost ─────────────────────────┐
│                                                                  │
│  The connection to the receiver was interrupted.                 │
│                                                                  │
│  Transferred          3.24 GB / 6.51 GB                          │
│  Files                152 / 391                                  │
│                                                                  │
│  Possible causes:                                                │
│  · Receiver closed the application                               │
│  · Network timeout (no data for 30s)                             │
│  · Firewall or NAT blocked the connection                        │
│                                                                  │
│                                                                  │
│  ❯ Retry session                                                 │
│    Abort transfer                                                │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

- Title: `Connection Lost` (bold, not red)
- Explanation: dim, limited to 3 lines maximum
- Metrics: dim, for context only
- Causes: dim, bullet points using `·`
- Actions: menu with two choices, same arrow-key selection behavior

### 5.2 Session Expired

```
┌──────────────────────── Session Expired ─────────────────────────┐
│                                                                  │
│  Session WHITE-8JKD-Q2L9 expired at 14:32:05.                    │
│                                                                  │
│  No receiver connected before the 5-minute window.               │
│                                                                  │
│  Start a new session to try again.                               │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

- No menu. Single status message.
- The app returns to the CLI prompt after a 2-second pause.

### 5.3 Verification Rejected

```
┌────────────────────── Connection Rejected ───────────────────────┐
│                                                                  │
│  Receiver rejected the connection.                               │
│                                                                  │
│  The fingerprint did not match or the user declined.             │
│                                                                  │
│  Session WHITE-8JKD-Q2L9 is now invalid.                         │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### 5.4 Hash Mismatch

```
┌────────────────────── Verification Failed ───────────────────────┐
│                                                                  │
│  File hash mismatch detected.                                    │
│                                                                  │
│  The transferred data does not match the source.                 │
│  Do not trust this transfer.                                     │
│                                                                  │
│  Possible causes:                                                │
│  · Data corruption during transfer                               │
│  · Man-in-the-middle attack                                      │
│  · Storage error on either end                                   │
│                                                                  │
│  Transferred files have been saved but may be                    │
│  incomplete or tampered with.                                    │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

- Warning tone could be emitted if the terminal supports bell (`\a`)
- Layout stays visible until user presses any key

---

## 6. Selection Menu Behavior

### 6.1 Rendering

```
  ❯ Yes, approve connection         ← bold (selected)
    No, reject connection            ← dim
    Abort session                    ← dim
```

- `❯` prefix on selected item only
- Selected item: bold
- Unselected: dim
- Menu rendered at the same vertical position regardless of selection length

### 6.2 Interaction

| Key | Behavior |
|-----|----------|
| `↑` / `k` | Move selection up, wrap to last |
| `↓` / `j` | Move selection down, wrap to first |
| `Enter` | Confirm selection |
| `Ctrl+C` | Abort session, return to shell |

- Single-character shortcuts NOT shown. This is not an interactive dialog — it's a menu. Users who know vim keys can use j/k; others use arrows.
- No mouse support. This is a terminal.

### 6.3 Where menus appear

- Device Verification screen (3 options)
- Connection Lost screen (2 options)

---

## 7. Progress Bar Design

### 7.1 Style

```
████████████████████████░░░░░░░░░
```

- Filled: `█` (U+2588)
- Unfilled: `░` (U+2591)
- Width: exactly half the inner box width, computed as `(box_width - 4) / 2`
- Percentage: bold, right-aligned, 3 characters, e.g. ` 74%`
- Bar + percentage always on one line

### 7.2 Behavior

- Direction: left-to-right
- Resolution: resolves to block granularity (if 50 blocks, each block = 2%)
- Transfer progress: bytes received / total bytes
- Hash verification (complete screen): independent bar that fills during final check
- Integrity bar (transfer screen): same filled/unfilled pattern, label on same line

### 7.3 When to show a bar vs. a spinner

| Scenario | Widget |
|----------|--------|
| Session waiting for receiver | Spinner (single rotating character) |
| Scanning files | Status text + count |
| Data transfer | Progress bar |
| Hash verification | Progress bar |
| Encryption handshake | Status text only |

The spinner character is `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` (Braille pattern — no custom characters).

---

## 8. Typography Rules

### 8.1 Font selection

- Application does not set fonts. Respects terminal emulator's monospace font.
- Design assumes standard 8-character-wide glyphs.

### 8.2 Weight usage

| Weight | Where |
|--------|-------|
| Normal | All body text, values, file paths |
| Dim | Labels, explanations, status text, unselected menu items |
| Bold | Session code, percentage, status (Success/Failed), selected menu item, fingerprint |

- No italic. Terminal italic is inconsistently rendered.
- No underline (reserved for links, which project white does not use).

### 8.3 Alignment

| Alignment | Where |
|-----------|-------|
| Center | Layout titles, session code, status lines, box borders |
| Left | All body content, labels, file paths |
| Right | Percentage on progress bar line, timers |

---

## 9. Spacing Rules

### 9.1 Grid

The application uses a 2-character horizontal grid unit and a 1-line vertical grid.

```
Horizontal: 2-space indentation from box edges
Vertical:   1 blank line between logical groups
```

### 9.2 Box padding

```
┌──────────────────────────────────────────────────────┐
│  text-content-here (2 spaces from left)              │
│                                                      │
│  grouped sections have 1 blank line between them     │
│                                                      │
└──────────────────────────────────────────────────────┘
```

### 9.3 Column layout

Multi-column rows (Files / Speed / ETA) use fixed column positions:

```
  Files             248 / 391
  ^^^^^             ^^^^^^^^^
  6-char label      12-char value (right-padded)
```

Labels are right-padded to 16 characters total (label + spaces + value).

---

## 10. Border Style Guide

### 10.1 Characters

```
┌  U+250C  ─  U+2500  ┐  U+2510  │  U+2502
├  U+251C  ┤  U+2524  └  U+2514  ┘  U+2518
```

- Only these 8 characters. No double-line borders (`╔═╗`), no rounded corners (`╭─╮`).
- Dividers: `├─┤` (single-line T-junctions), used to visually separate sections within a layout

### 10.2 Where borders appear

| Element | Border |
|---------|--------|
| Each layout | Full box (`┌─┐ │ └─┘`) |
| Section dividers within a layout | `├─┤` |
| Content | No borders |

- Never nest boxes.
- Never box individual rows.

### 10.3 Width

Box width: **exactly the terminal width** minus 2 characters (1 space padding on each side becomes the margin).

```
Terminal width = 80

┌───────────────────────────────────────────────────────┐
│  (76 characters of content)                           │
└───────────────────────────────────────────────────────┘
```

This ensures the box fills the terminal naturally on any window size.

---

## 11. Color Usage Guidelines

### 11.1 Palette

No colors. The application uses only:

| Attribute | ANSI Code | Usage |
|-----------|-----------|-------|
| Default | (inherit) | All body content |
| Bold | `\e[1m` | Focal elements (code, percentage, status, selected item) |
| Dim | `\e[2m` | Labels, instructions, explanations, unselected items |

### 11.2 Rationale

- Color perception varies (colorblindness, dark/light terminals, solarized vs. default)
- Colors imply meaning (red = danger, green = success) that may not match severity
- Project White is a tool for moving data — confidence comes from clarity, not coloring
- Dim and bold work universally across every terminal theme

### 11.3 Exception

The application **may** use terminal colors if the user explicitly sets `PW_THEME=light` or `PW_THEME=dark`, but v1 ships without this feature.

---

## 12. Update/Animation Behavior

### 12.1 Core principle

**No animated transitions.** Layouts replace each other atomically.

### 12.2 Single-line updates

Within a stable layout, values update by overwriting the same line:

```
\r\e[K<new content>
```

- `\r` returns cursor to column 0
- `\e[K` clears to end of line
- This prevents flicker from full-screen clears

### 12.3 Progress bar updates

The progress bar and percentage update on the same line using the mechanism above. The bar refills from the left — no wipe-and-redraw.

### 12.4 State transitions (Layout → Layout)

```
\e[2J\e[H    (clear screen + home cursor)
...print new layout...
```

- Full clear is acceptable only when the entire layout changes (3 times per session)
- No fade, no wipe, no slide — instant replacement

### 12.5 File name scroll

Current file names longer than the available column width do not scroll or marquee. Instead, the path is truncated from the left with `…`:

```
  Current File
  …/network/session.rs
```

---

## 13. Accessibility Considerations

### 13.1 Screen reader compatibility

- All content is text. No graphical elements.
- Progress bars use only Unicode characters accessible to screen readers (`█` → "block", `░` → "shade")
- The application does not depend on color

### 13.2 Keyboard navigation

- Arrow keys and vim keys (`j`/`k`) for menus
- Enter to confirm
- `Ctrl+C` as universal abort
- No hidden keyboard shortcuts

### 13.3 Font size

- All dimensions are character-relative (not pixel-relative)
- Works at any terminal font size
- No minimum width requirement below 60 columns

### 13.4 Terminal width adaptation

- Below 60 columns: the application prints a single-line warning and waits for resize
- 60–120 columns: standard layout (box fills width)
- Above 120 columns: box is capped at 120 characters wide, centered

---

## 14. CLI Interaction Guidelines

### 14.1 Commands

```
pw send [<path>]              Start as sender (default: cwd)
pw receive                     Start as receiver
pw receive [--in <path>]      Join existing session
pw receive <code> [--in <path>]  Join session, save to directory
pw update                     Update to latest version from server
pw update [--server <url>]    Update from custom signaling server
pw --version                  Print version
pw --help                     Print help
```

### 14.2 Output behavior during transfer

- `stderr` is reserved for logging/debug only
- `stdout` is untouched by the TUI
- When `--debug` is passed, the application skips the TUI entirely and writes flat logs to stderr

### 14.3 Abort

- `Ctrl+C` at any point: close connections, delete session on server, print one-line confirmation to stderr, exit with code 1
- No prompt "Are you sure?" — if they hit Ctrl+C they mean it

### 14.4 Exit codes

| Scenario | Code |
|----------|------|
| Transfer completed successfully | 0 |
| User aborted (Ctrl+C) | 1 |
| Connection lost / timeout | 2 |
| Verification failure | 3 |
| Session expired | 4 |
| Internal error | 5 |

---

## 15. Rationale Index

Every significant design decision:

| # | Decision | Why |
|---|----------|-----|
| 15.1 | No colors | Works on every terminal theme, no ambiguity for colorblind users, avoids false signaling |
| 15.2 | Three layout replacement | Prevents context loss from scrolling; spatial memory helps users find information faster on repeat use |
| 15.3 | Session code as focal point | It's the only shared secret — everything else depends on it; should be the first thing a user sees and the last thing they forget |
| 15.4 | Progress bar always half box width | Wide enough to read resolution, narrow enough to leave room for percentage on same line without wrapping |
| 15.5 | Dim labels | Creates clear information hierarchy without colors; labels are scaffolding, not content |
| 15.6 | Two verification indicators only | Each answers a distinct question: "did the hash match?" and "did all bytes arrive?" — adding more would reduce trust in both |
| 15.7 | Compression section hides when irrelevant | Empty sections create visual noise; conditional rendering keeps the layout dense with signal |
| 15.8 | Menu without letter shortcuts | Arrow-key menus are discoverable; letter shortcuts require memorization or on-screen hints that add clutter |
| 15.9 | Full clear on layout change | In-band updates within a layout are smooth; layout-level changes are infrequent enough that a clear is acceptable and simpler to reason about |
| 15.10 | No emoji | Emoji render differently across terminals, fonts, and OS versions; a `✓` in one terminal is a garbled box in another. Text-only is reliable |
| 15.11 | Braille spinner for waiting state | Spinner is a universally understood "not stuck, just waiting" signal that uses one character position |
| 15.12 | Left-truncated long paths | The filename (basename) is the most important part; the directory path is context that can be inferred |
| 15.13 | Box width = terminal width − 2 | Creates a natural margin; keeps the layout adaptive without reflow logic |
| 15.14 | No CPU/RAM/threads display | The user cannot act on those metrics; they add visual weight without answering a question the user has during a transfer |
| 15.15 | Single progress bar line | Avoids the "multiple bars at different speeds" problem; one unified bar is truthful and stable |
| 15.16 | Exit codes for scripting | Non-interactive use (CI, cron) needs machine-readable results; 0/1 is not enough for a tool with distinct failure modes |
| 15.17 | No "Are you sure?" on Ctrl+C | A transfer tool must respect the user's intent to stop immediately; a confirmation dialog during a 6 GB transfer creates frustration, not safety |

---

## 16. Future Extensions (v2+)

These are explicitly **not** part of v1 but noted for architectural awareness:

- Resumable transfers (requires session persistence)
- `PW_THEME` for optional light/dark color support
- JSON output mode (`--json`) for programmatic consumers
- Multi-peer sessions (one sender, N receivers)
- Progress bar customization (style, width)
- Transfer queue (sequential or parallel)
- Bandwidth throttle indicator
- Native notifications on completion (`pw notify` integration)
