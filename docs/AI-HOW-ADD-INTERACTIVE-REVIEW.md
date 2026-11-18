# GIC Review API: how to add an interactive review

**For:** an AI assistant (or script) that publishes interactive review pages to GIC Review
**through the REST API only**. You have no access to the source code and don't need it.
Typical job: *"Turn this PDF lecture into an interactive review page for the HCI subject."*

An **interactive review** is one self-contained `.html` file (a study guide with sections,
summaries, diagrams, collapsible parts and small self-checks) attached to a subject. Students
open it from the subject's page. Everything you need is in this file. (Quizzes: see
`AI-HOW-ADD-QUIZ.md`.)

---

## 1. Quick start

You need two values from the human:

| Value | Example | Notes |
|---|---|---|
| `BASE` (API base URL) | `https://api.example.com/api` | Every endpoint below is `BASE` + path. |
| `KEY` (API key) | `gic_3f9a…` (68 chars) | Made by a manager in **Dashboard → API key**, permission **`reviews:write`**. Secret: never show it or put it in the HTML. |

Every request that needs the key sends this header:

```
Authorization: Bearer KEY
```

Test it:

```bash
curl -H "Authorization: Bearer $KEY" $BASE/auth/me
# 200 → { "role": "manager", "auth_method": "api_key", "api_key_scopes": ["reviews:write"], … }
```

`401` → the key is wrong, expired or revoked. Stop and ask the human for a new key.

---

## 2. Endpoint list (everything you may call)

Full URL = `BASE` + path. `{id}` = a review id.

| # | Method | Path | Key? | Use it to… |
|---|---|---|---|---|
| 1 | `GET` | `/auth/me` | yes | check the key works and has `reviews:write` |
| 2 | `GET` | `/subjects` | no | list subjects → get the subject's numeric `id` |
| 3 | `GET` | `/admin/reviews?subject_id={n}` | yes | list reviews you can manage (avoid duplicates, find ids) |
| 4 | `GET` | `/admin/reviews/{id}` | yes | read one review (its public `url`, title, status) |
| 5 | `POST` | `/admin/uploads/image` | yes | upload an image for the page → use the returned `url` in `<img>` |
| 6 | `POST` | `/admin/reviews` | yes | **upload a new review page** ← main endpoint |
| 7 | `PUT` | `/admin/reviews/{id}` | yes | replace the HTML and/or change title, slug, order, published |
| 8 | `DELETE` | `/admin/reviews/{id}` | yes | delete a review (owner only; **ask the human first**) |
| 9 | `GET` | `/subjects/{slug}/reviews` | no | see what students see (published reviews only) |

**Most jobs only need #1, #2, #3, #5 (if there are images) and #6.**

Don't call anything else under `/admin/…` (users, subjects, API keys, sharing). Those are for
humans in the dashboard and refuse API keys (`403 api_key_not_allowed`). There is no endpoint to
create subjects: if the subject you need is missing, ask the human.

**What your key can reach:** it acts as the manager who made it. You see and edit reviews that
manager owns or that another manager shared with them; others return `404`. On a shared review
you can update it (#7) but not delete it (`403 owner_only`).

---

## 3. Recipe: create a review page from a PDF

**Step 1. Check the key** (#1). Expect `"reviews:write"` in `"api_key_scopes"`.

**Step 2. Find the subject id** (#2)

```bash
curl $BASE/subjects
# [ { "id": 1, "slug": "hci", "name": "Human Computer Interaction", "has_review": false, … } ]
```

**Step 3. Check for an existing review** (#3)

```bash
curl -H "Authorization: Bearer $KEY" "$BASE/admin/reviews?subject_id=1"
# [ { "id": 7, "title": "HCI: Usability Heuristics", "slug": "usability-heuristics", "is_published": true, … } ]
```
Same topic already there? Ask whether to **update it** (#7) instead of creating a new one.

**Step 4. Upload images, if any** (#5). Small diagrams can be inline `<svg>` or `data:` URIs
instead. For figures from the PDF (exported as PNG, cropped):

```bash
curl -H "Authorization: Bearer $KEY" -F "file=@figure-3.png" $BASE/admin/uploads/image
# 201 → { "key": "tmp/2f1c…png", "url": "https://files.example.com/tmp/2f1c…png" }
```
Use the **`url`** in the page: `<img src="https://files.example.com/tmp/2f1c…png" alt="…">`.

**Step 5. Write the HTML.** Copy the template in section 6, keep its `<style>` and `<script>`
theme block **unchanged**, and replace the content inside `<main>`. Follow the rules in section 5.

**Step 6. Upload it as a draft** (#6)

```bash
curl -H "Authorization: Bearer $KEY" \
  -F "subject_id=1" \
  -F "title=HCI: Usability Heuristics" \
  -F "slug=usability-heuristics" \
  -F "is_published=false" \
  -F "file=@usability-heuristics.html;type=text/html" \
  $BASE/admin/reviews
# 201 → { "id": 7, "url": "https://files.example.com/reviews/1/….html?v=…", "is_published": false, … }
```
`409` → that `slug` is already used in this subject: choose another one.

**Step 7. Report back** to the human: review `id`, `title` and `url` (they can open the `url` to
check it). Students can't see it yet.

**Step 8. Publish only if the human says so** (#7)

```bash
curl -H "Authorization: Bearer $KEY" -X PUT -F "is_published=true" $BASE/admin/reviews/7
```

---

## 4. Endpoint details

### #1 `GET /auth/me`
Response `200`: `{ "id", "email", "display_name", "role": "manager", "auth_method": "api_key", "api_key_scopes": [...] }`.

### #2 `GET /subjects` (no key)
Response `200`: array of `{ "id", "slug", "name", "description", "has_review", "quiz_count", … }`.

### #3 `GET /admin/reviews?subject_id={n}`
`subject_id` optional. Response `200`: array of **review objects** (below), unpublished included.

### #4 `GET /admin/reviews/{id}`
Response `200`: one review object. `404` if it doesn't exist or isn't yours.

### #5 `POST /admin/uploads/image`
`multipart/form-data`, field `file` (PNG, JPEG, WebP or GIF; max 5 MB by default). Response
`201`: `{ "key", "url" }`. Use `url` in your HTML. Resized to max 1600 px wide.

### #6 `POST /admin/reviews`
`multipart/form-data`:

| Field | Required | Meaning |
|---|---|---|
| `file` | yes | the page: file name ending in `.html`, type `text/html`, UTF-8, max 10 MB by default |
| `subject_id` | yes | numeric subject id from #2 |
| `title` | yes | shown in the review list and page header (≤ 200 chars) |
| `slug` | no | short id in the page link, `lowercase-with-dashes`, unique per subject; default: made from the title |
| `sort_order` | no | position in the subject's list (number, default 0, lowest first) |
| `is_published` | no | `true` or `false` (default `false`) |

Response `201`: the review object.

### #7 `PUT /admin/reviews/{id}`
Same fields as #6, **all optional**. Send only what changes; the rest stays as it is. A new
`file` replaces the page **at the same address** (the returned `url` gets a new `?v=` so
browsers load the new version).

```bash
curl -H "Authorization: Bearer $KEY" -X PUT \
  -F "file=@usability-heuristics-v2.html;type=text/html" \
  $BASE/admin/reviews/7
```
Response `200`: the review object.

### #8 `DELETE /admin/reviews/{id}`
Deletes the review and its file. Response `204`. **Owner only.** Never do this without the
human's explicit request.

### #9 `GET /subjects/{slug}/reviews` (no key)
Published reviews of the subject, as students see them.

### The review object (returned by #3, #4, #6, #7)

```jsonc
{
  "id": 7,
  "subject_id": 1,
  "title": "HCI: Usability Heuristics",
  "slug": "usability-heuristics",
  "url": "https://files.example.com/reviews/1/3f9a….html?v=1763458000",   // the public page
  "sort_order": 0,
  "is_published": false,
  "access": "owner", "owner_id": 4, "owner_name": "Dara", "share_count": 0,
  "created_at": "2026-11-18T10:00:00", "updated_at": "2026-11-18T10:00:00"
}
```

On the website, students find it at `/subjects/<subject-slug>/review` (with `?r=<slug>` when the
subject has several reviews).

---

## 5. Rules for the HTML

The page runs in `<iframe sandbox="allow-scripts">` **without same-origin access**, served
from the file storage domain:

| ✅ Do | ❌ Don't |
|---|---|
| One file: inline `<style>` and `<script>`, `<meta charset="utf-8">`, `<meta name="viewport" …>` | Link external CSS/JS/fonts from other sites, analytics, trackers or ads |
| Interactivity with plain JavaScript (accordions, tabs, flip cards, mini quizzes, SVG/canvas) | Use `localStorage`, `sessionStorage`, IndexedDB or cookies (they fail in the sandbox) |
| Keep state in JS variables | Call the GIC Review API from the page (no credentials, CORS refuses it) |
| Responsive layout (fluid widths, `max-width`, works on phones) | Open popups, submit forms or navigate the top window (not allowed by the sandbox) |
| Use the Catppuccin CSS variables from the template (`var(--ctp-text)`, `var(--ctp-accent)`, …) | Hard-code colors that only work on a light or only on a dark background |
| UTF-8, a real `<html>` document, file name ending in `.html` | Exceed the size limit (default 10 MB) |
| Accessible: headings in order, `alt` on images, buttons as `<button>` | Copy large copyrighted text verbatim; summarise and explain instead |

**Theme:** students can pick one of four Catppuccin flavors (Latte light, Frappé, Macchiato,
Mocha dark) and an accent color. The site sends the choice to your page with
`postMessage({ type: "gic-theme", flavor: "mocha", accent: "mauve" })`. The template already
listens for it and falls back to the device's light/dark setting. Keep that block unchanged and
style everything with the `--ctp-*` variables so the review always matches the site.

Available variables: `--ctp-base` (page background), `--ctp-mantle` / `--ctp-crust` (panels),
`--ctp-surface0/1/2` (cards, borders), `--ctp-overlay0/1/2` (muted), `--ctp-text`,
`--ctp-subtext0/1` (text), `--ctp-accent` (links, highlights) and the colors `--ctp-green`
(correct), `--ctp-red` (wrong), `--ctp-yellow` (warning), `--ctp-blue` (info), plus
`rosewater, flamingo, pink, mauve, maroon, peach, teal, sky, sapphire, lavender`.

---

## 6. Starter template (copy this)

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>HCI — Interactive Review (starter)</title>
    <!--
      GIC Review interactive review starter template.
      - One self-contained file: inline CSS/JS, images as data: URIs or absolute R2 public URLs.
      - Runs in <iframe sandbox="allow-scripts"> with NO same-origin access:
        no localStorage/cookies from the parent, no calls to the GIC Review API, no trackers.
      - Theme: the parent sends postMessage({ type: "gic-theme", flavor, accent }).
        Without a message, it follows prefers-color-scheme (Latte light / Mocha dark).
    -->
    <style>
      /* ---- Catppuccin palette (generated from @catppuccin/palette) ---- */
      :root, [data-flavor="latte"] {
        --ctp-rosewater: #dc8a78;
        --ctp-flamingo: #dd7878;
        --ctp-pink: #ea76cb;
        --ctp-mauve: #8839ef;
        --ctp-red: #d20f39;
        --ctp-maroon: #e64553;
        --ctp-peach: #fe640b;
        --ctp-yellow: #df8e1d;
        --ctp-green: #40a02b;
        --ctp-teal: #179299;
        --ctp-sky: #04a5e5;
        --ctp-sapphire: #209fb5;
        --ctp-blue: #1e66f5;
        --ctp-lavender: #7287fd;
        --ctp-text: #4c4f69;
        --ctp-subtext1: #5c5f77;
        --ctp-subtext0: #6c6f85;
        --ctp-overlay2: #7c7f93;
        --ctp-overlay1: #8c8fa1;
        --ctp-overlay0: #9ca0b0;
        --ctp-surface2: #acb0be;
        --ctp-surface1: #bcc0cc;
        --ctp-surface0: #ccd0da;
        --ctp-base: #eff1f5;
        --ctp-mantle: #e6e9ef;
        --ctp-crust: #dce0e8;
        color-scheme: light;
      }
      [data-flavor="frappe"] {
        --ctp-rosewater: #f2d5cf;
        --ctp-flamingo: #eebebe;
        --ctp-pink: #f4b8e4;
        --ctp-mauve: #ca9ee6;
        --ctp-red: #e78284;
        --ctp-maroon: #ea999c;
        --ctp-peach: #ef9f76;
        --ctp-yellow: #e5c890;
        --ctp-green: #a6d189;
        --ctp-teal: #81c8be;
        --ctp-sky: #99d1db;
        --ctp-sapphire: #85c1dc;
        --ctp-blue: #8caaee;
        --ctp-lavender: #babbf1;
        --ctp-text: #c6d0f5;
        --ctp-subtext1: #b5bfe2;
        --ctp-subtext0: #a5adce;
        --ctp-overlay2: #949cbb;
        --ctp-overlay1: #838ba7;
        --ctp-overlay0: #737994;
        --ctp-surface2: #626880;
        --ctp-surface1: #51576d;
        --ctp-surface0: #414559;
        --ctp-base: #303446;
        --ctp-mantle: #292c3c;
        --ctp-crust: #232634;
        color-scheme: dark;
      }
      [data-flavor="macchiato"] {
        --ctp-rosewater: #f4dbd6;
        --ctp-flamingo: #f0c6c6;
        --ctp-pink: #f5bde6;
        --ctp-mauve: #c6a0f6;
        --ctp-red: #ed8796;
        --ctp-maroon: #ee99a0;
        --ctp-peach: #f5a97f;
        --ctp-yellow: #eed49f;
        --ctp-green: #a6da95;
        --ctp-teal: #8bd5ca;
        --ctp-sky: #91d7e3;
        --ctp-sapphire: #7dc4e4;
        --ctp-blue: #8aadf4;
        --ctp-lavender: #b7bdf8;
        --ctp-text: #cad3f5;
        --ctp-subtext1: #b8c0e0;
        --ctp-subtext0: #a5adcb;
        --ctp-overlay2: #939ab7;
        --ctp-overlay1: #8087a2;
        --ctp-overlay0: #6e738d;
        --ctp-surface2: #5b6078;
        --ctp-surface1: #494d64;
        --ctp-surface0: #363a4f;
        --ctp-base: #24273a;
        --ctp-mantle: #1e2030;
        --ctp-crust: #181926;
        color-scheme: dark;
      }
      [data-flavor="mocha"] {
        --ctp-rosewater: #f5e0dc;
        --ctp-flamingo: #f2cdcd;
        --ctp-pink: #f5c2e7;
        --ctp-mauve: #cba6f7;
        --ctp-red: #f38ba8;
        --ctp-maroon: #eba0ac;
        --ctp-peach: #fab387;
        --ctp-yellow: #f9e2af;
        --ctp-green: #a6e3a1;
        --ctp-teal: #94e2d5;
        --ctp-sky: #89dceb;
        --ctp-sapphire: #74c7ec;
        --ctp-blue: #89b4fa;
        --ctp-lavender: #b4befe;
        --ctp-text: #cdd6f4;
        --ctp-subtext1: #bac2de;
        --ctp-subtext0: #a6adc8;
        --ctp-overlay2: #9399b2;
        --ctp-overlay1: #7f849c;
        --ctp-overlay0: #6c7086;
        --ctp-surface2: #585b70;
        --ctp-surface1: #45475a;
        --ctp-surface0: #313244;
        --ctp-base: #1e1e2e;
        --ctp-mantle: #181825;
        --ctp-crust: #11111b;
        color-scheme: dark;
      }
      :root { --ctp-accent: var(--ctp-mauve); }
      [data-accent="rosewater"] { --ctp-accent: var(--ctp-rosewater); }
      [data-accent="flamingo"] { --ctp-accent: var(--ctp-flamingo); }
      [data-accent="pink"] { --ctp-accent: var(--ctp-pink); }
      [data-accent="mauve"] { --ctp-accent: var(--ctp-mauve); }
      [data-accent="red"] { --ctp-accent: var(--ctp-red); }
      [data-accent="maroon"] { --ctp-accent: var(--ctp-maroon); }
      [data-accent="peach"] { --ctp-accent: var(--ctp-peach); }
      [data-accent="yellow"] { --ctp-accent: var(--ctp-yellow); }
      [data-accent="green"] { --ctp-accent: var(--ctp-green); }
      [data-accent="teal"] { --ctp-accent: var(--ctp-teal); }
      [data-accent="sky"] { --ctp-accent: var(--ctp-sky); }
      [data-accent="sapphire"] { --ctp-accent: var(--ctp-sapphire); }
      [data-accent="blue"] { --ctp-accent: var(--ctp-blue); }
      [data-accent="lavender"] { --ctp-accent: var(--ctp-lavender); }

      /* ---- Layout (responsive, mobile first) ---- */
      * { box-sizing: border-box; }
      body {
        margin: 0;
        font: 16px/1.6 system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
        background: var(--ctp-base);
        color: var(--ctp-text);
      }
      main { max-width: 820px; margin: 0 auto; padding: 24px 16px 64px; }
      h1 { margin: 0 0 4px; font-size: 1.8rem; }
      h2 { margin-top: 2rem; color: var(--ctp-accent); }
      .lead { color: var(--ctp-subtext0); margin-top: 0; }
      details {
        background: var(--ctp-mantle);
        border: 1px solid var(--ctp-surface0);
        border-radius: 12px;
        padding: 12px 16px;
        margin: 12px 0;
      }
      summary { cursor: pointer; font-weight: 600; }
      .callout {
        border-left: 4px solid var(--ctp-blue);
        background: color-mix(in srgb, var(--ctp-blue) 12%, transparent);
        padding: 10px 14px;
        border-radius: 8px;
      }
      button.choice {
        display: block;
        width: 100%;
        text-align: left;
        margin: 6px 0;
        padding: 10px 12px;
        border-radius: 10px;
        border: 2px solid var(--ctp-surface1);
        background: var(--ctp-surface0);
        color: var(--ctp-text);
        font: inherit;
        cursor: pointer;
      }
      button.choice.correct { border-color: var(--ctp-green); }
      button.choice.wrong { border-color: var(--ctp-red); }
      .feedback { min-height: 1.6em; font-weight: 600; }
      code { background: var(--ctp-surface0); padding: 1px 6px; border-radius: 6px; }
    </style>
    <script>
      // ---- Theme: prefers-color-scheme until the parent tells us otherwise ----
      (function () {
        var FLAVORS = ["latte", "frappe", "macchiato", "mocha"];
        var ACCENTS = ["rosewater", "flamingo", "pink", "mauve", "red", "maroon", "peach",
          "yellow", "green", "teal", "sky", "sapphire", "blue", "lavender"];
        var root = document.documentElement;
        var fromParent = false;
        var media = window.matchMedia("(prefers-color-scheme: dark)");

        function applySystem() {
          if (!fromParent) root.setAttribute("data-flavor", media.matches ? "mocha" : "latte");
        }
        applySystem();
        media.addEventListener("change", applySystem);

        window.addEventListener("message", function (event) {
          // Only accept theme messages from the embedding GIC Review page.
          if (event.source !== window.parent) return;
          var data = event.data;
          if (!data || data.type !== "gic-theme") return;
          if (FLAVORS.indexOf(data.flavor) >= 0) {
            fromParent = true;
            root.setAttribute("data-flavor", data.flavor);
          }
          if (ACCENTS.indexOf(data.accent) >= 0) root.setAttribute("data-accent", data.accent);
        });
      })();
    </script>
  </head>
  <body>
    <main>
      <h1>Human Computer Interaction</h1>
      <p class="lead">Interactive review: usability principles you must know for the exam.</p>

      <h2>1. Nielsen's 10 usability heuristics</h2>
      <details open>
        <summary>Visibility of system status</summary>
        <p>Keep users informed about what is going on with timely, appropriate feedback.</p>
      </details>
      <details>
        <summary>Error prevention</summary>
        <p>Design to prevent problems: confirmations for destructive actions, constraints, good defaults.</p>
      </details>

      <h2>2. Fitts's law</h2>
      <p class="callout">Time to reach a target = <code>a + b · log2(D / W + 1)</code>: bigger, closer targets are faster.</p>

      <h2>3. Quick check</h2>
      <p>Which change makes a button faster to click according to Fitts's law?</p>
      <div id="quiz">
        <button class="choice" data-correct="false">Change its color to red</button>
        <button class="choice" data-correct="true">Make it bigger and place it closer to the cursor</button>
        <button class="choice" data-correct="false">Add a tooltip</button>
      </div>
      <p class="feedback" id="feedback" aria-live="polite"></p>
    </main>
    <script>
      // Interactivity works fine inside the sandbox (no storage, no network needed).
      document.querySelectorAll("#quiz .choice").forEach(function (btn) {
        btn.addEventListener("click", function () {
          var ok = btn.getAttribute("data-correct") === "true";
          btn.classList.add(ok ? "correct" : "wrong");
          var fb = document.getElementById("feedback");
          fb.textContent = ok ? "Correct!" : "Not quite, try again.";
          fb.style.color = ok ? "var(--ctp-green)" : "var(--ctp-red)";
        });
      });
    </script>
  </body>
</html>
```

---

## 7. Errors

Errors look like `{ "error": { "code": "…", "message": "…" } }`. The `message` says what to fix.

| HTTP | `code` | What to do |
|---|---|---|
| 401 | `unauthorized` | Key missing/wrong/expired/revoked → ask the human for a new key. |
| 403 | `missing_scope` | Key lacks `reviews:write` → ask for a key with that permission. |
| 403 | `owner_only` | Only the review's owner can do that (e.g. delete a shared review). |
| 403 | `api_key_not_allowed` | You called a dashboard-only endpoint; don't. |
| 404 | `not_found` | Wrong id, or not your review. |
| 409 | `conflict` | That `slug` already exists in this subject → choose another. |
| 413 | `payload_too_large` | File too big. |
| 415 | `unsupported_media_type` | Not an `.html` file / not `text/html` (or not a supported image for #5). |
| 422 | `validation_error` | Bad field (not UTF-8, not an HTML document, bad slug, …) → fix and retry. |

---

## 8. Final checklist

- [ ] One self-contained `.html`; no external scripts/styles/fonts/trackers; no storage or cookies.
- [ ] Theme block from the template unchanged; all colors use `var(--ctp-…)`.
- [ ] Works on a phone-sized screen; every image has `alt` text.
- [ ] Uploaded as a draft unless the human asked to publish; didn't duplicate an existing review.
- [ ] Told the human the review `id`, `title` and `url`. Never reveal the key.
