# GIC Review API: how to add an interactive review

**Audience:** an AI assistant (or any script) that publishes interactive reviews to GIC Review
**through the REST API only**. You do not have, and do not need, access to the source code. A
typical job: *"Turn this PDF lecture into an interactive review page for the HCI subject."*

This document is self-contained. Quizzes are covered in the companion document
`AI-HOW-ADD-QUIZ.md`.

---

## 1. What an interactive review is

A **single, self-contained `.html` file**: a study guide for one subject with sections,
summaries, diagrams, collapsible details and small self-checks. Students open it from the
subject's page; it is shown in a sandboxed frame, styled with the site's colors.

## 2. What you need from the human

| Value | Example | Where it comes from |
|---|---|---|
| **API base URL** | `https://api.example.com/api` | The site owner. Every path below is relative to it. |
| **API key** | `gic_3f9a…` (68 characters) | A **manager** creates it in **Dashboard → API key**, permission `reviews:write`. |

```bash
export GIC_API=https://api.example.com/api   # no trailing slash
export GIC_KEY=gic_…                         # keep it secret
```

Send `Authorization: Bearer <API key>` on every request. Never print the key in answers or
put it in the HTML. `401` means the key is wrong, expired or revoked: ask for a new one.

**What the key can do:** it acts as the manager who created it. You can manage reviews that
manager **owns** or that **another manager shared with them**. Other managers' reviews are
invisible (`404`). On a shared review you can update it but **not delete** it
(`403 owner_only`). Sharing, users, subjects and keys are handled by humans in the dashboard
(`403 api_key_not_allowed`). If the subject you need doesn't exist, ask the human (admins
create subjects).

---

## 3. Recommended workflow (e.g. from a PDF)

1. `GET /auth/me`: check `"api_key_scopes"` contains `"reviews:write"`.
2. `GET /subjects` (public): note the subject's numeric `id`.
3. `GET /admin/reviews?subject_id=<id>`: see what already exists, so you don't create duplicates.
   To update an existing review, use `PUT` (section 6) instead of creating a new one.
4. Write the HTML: start from the **template in section 5**, keep its `<style>`/`<script>` theme
   block, and replace the `<main>` content with the material, following the rules in section 4.
5. Images: small diagrams can be inline `data:` URIs or inline `<svg>`. For larger figures,
   upload them with `POST /admin/uploads/image` (multipart field `file`) and use the returned
   absolute `url` in `<img src="…">`.
6. Upload: `POST /admin/reviews` with `is_published=false` first.
7. Report the review `id`, `title` and `url` to the human; publish when asked
   (`PUT` with `is_published=true`).

---

## 4. Rules for the HTML

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

## 5. Starter template (copy this)

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

## 6. Endpoints

All need `Authorization: Bearer <key>` with `reviews:write`.

### Create: `POST /admin/reviews` (`multipart/form-data`)

| Field | Required | Meaning |
|---|---|---|
| `file` | yes | the `.html` file, content type `text/html`, UTF-8 |
| `subject_id` | yes | numeric subject id from `GET /subjects` |
| `title` | yes | shown in the list and the page header (≤ 200 chars) |
| `slug` | no | `lowercase-with-dashes`, unique per subject; default: made from the title |
| `sort_order` | no | position in the subject's list (default 0, ascending) |
| `is_published` | no | `true` / `false` (default `false`); only published reviews are visible |

```bash
curl -H "Authorization: Bearer $GIC_KEY" \
  -F "subject_id=1" -F "title=HCI: Usability Heuristics" -F "slug=usability-heuristics" \
  -F "sort_order=0" -F "is_published=false" \
  -F "file=@usability-heuristics.html;type=text/html" \
  $GIC_API/admin/reviews
```

### Update / replace: `PUT /admin/reviews/{id}` (`multipart/form-data`)

Same fields, **all optional**; send only what changes. A new `file` replaces the HTML **at the
same address**, so links keep working (the returned `url` gets a new `?v=` to bypass caches).

```bash
curl -H "Authorization: Bearer $GIC_KEY" -X PUT \
  -F "file=@usability-heuristics-v2.html;type=text/html" -F "is_published=true" \
  $GIC_API/admin/reviews/7
```

### Read and delete

| Method & path | What it does |
|---|---|
| `GET /admin/reviews[?subject_id=1]` | Reviews you can manage (yours + shared with you), unpublished included. |
| `GET /admin/reviews/{id}` | One review. |
| `DELETE /admin/reviews/{id}` | Delete it and its file. **Owner only.** Ask the human first. |
| `GET /subjects/{slug}/reviews` | Public: what students see (published only). |
| `POST /admin/uploads/image` | Upload an image for the page → `{ "key", "url" }`; use `url`. |

**The review object:**

```jsonc
{
  "id": 7, "subject_id": 1, "title": "HCI: Usability Heuristics", "slug": "usability-heuristics",
  "url": "https://files.example.com/reviews/1/…html?v=1763458000",  // the public page
  "sort_order": 0, "is_published": false,
  "access": "owner", "owner_id": 4, "owner_name": "Dara", "share_count": 0,
  "created_at": "…", "updated_at": "…"
}
```

On the website, students open it at `/subjects/<subject-slug>/review` (with `?r=<slug>` when a
subject has several reviews).

---

## 7. Errors

Every error looks like `{ "error": { "code": "…", "message": "…" } }`.

| Status | `code` | Meaning / what to do |
|---|---|---|
| 401 | `unauthorized` | Missing, wrong, expired or revoked key. Ask for a new key. |
| 403 | `missing_scope` | The key lacks `reviews:write`. |
| 403 | `owner_only` | Only the owner may do this (e.g. delete a shared review). |
| 403 | `api_key_not_allowed` | Dashboard-only endpoint. |
| 404 | `not_found` | Doesn't exist, or you have no access to it. |
| 409 | `conflict` | That `slug` is already used in this subject; pick another. |
| 413 | `payload_too_large` | File too big. |
| 415 | `unsupported_media_type` | Not an `.html` file / not `text/html`. |
| 422 | `validation_error` | Invalid field (e.g. not UTF-8, not an HTML document, bad slug). |

---

## 8. Checklist before you report back

- [ ] Self-contained `.html`; no external scripts, styles, fonts or trackers; no storage/cookies.
- [ ] The theme block from the template is unchanged; colors use `--ctp-*` variables.
- [ ] Works on a phone-sized screen; images have `alt` text.
- [ ] Uploaded unpublished unless the human asked to publish; no duplicate of an existing review.
- [ ] You gave the human the review `id`, `title` and `url`. Never include the API key.
