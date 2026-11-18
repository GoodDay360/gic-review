# How to add an interactive review

Audience: AI assistants and developers. Read `prompt.md` and `docs/AI-SPEC.md` first.
If you change the HTML rules, the upload API or the theme protocol, update this file in
the same commit.

## What an interactive review is

A **single, self-contained `.html` file** for one subject: a study guide with
collapsible sections, diagrams, mini self-checks, and so on.

- All CSS and JS are **inline** (`<style>`, `<script>`).
- Images are either **inline `data:` URIs** or **uploaded to R2 first and referenced by
  their absolute public URL** (`https://<R2_PUBLIC_BASE_URL>/…`). To upload an image, use
  `POST /api/admin/uploads/image` (see `docs/AI-HOW-ADD-QUIZ.md`) and copy the returned `url`.
- The file is stored in R2 as `reviews/<subject_id>/<uuid>.html` and served from the
  **R2 public domain**, never from the main site.

Students open it at `/subjects/<subject-slug>/review`. If a subject has several published
reviews, they see a list first (`?r=<review-slug>` opens one).

## Rules for the HTML (sandbox)

The page is shown in:

```html
<iframe src="https://files.example.com/reviews/1/<uuid>.html" sandbox="allow-scripts"></iframe>
```

There is **no `allow-same-origin`**, so the review runs with an opaque origin. Therefore:

| ✅ Do | ❌ Don't |
|---|---|
| Use inline JS for interactivity (accordions, quizzes, tabs, canvas, SVG) | Rely on `localStorage`, `sessionStorage`, IndexedDB or cookies (they throw or are empty) |
| Keep all state in JS variables | Call the GIC Review API (`/api/...`): no cookies, CORS will refuse it |
| Make it responsive (fluid widths, `max-width`, `<meta name="viewport">`) | Load external trackers, analytics, ads or fonts from third-party CDNs |
| Use the Catppuccin CSS variables (template below) | Open popups or navigate the top window (`allow-popups`/`allow-top-navigation` are off) |
| Keep it under `MAX_REVIEW_HTML_SIZE_MB` (default 10 MB) | Submit forms (`allow-forms` is off) |
| Use UTF-8 and a real `<html>` document | Upload `.htm`, `.xhtml` or anything that isn't `.html` |

## Styling and theme sync

Use the **Catppuccin** palette so the review matches the site. Because the iframe is
sandboxed, it cannot read the parent's theme, so the parent sends it:

```js
// sent by frontend/src/views/ReviewView.vue on load and whenever the user changes theme
iframe.contentWindow.postMessage({ type: 'gic-theme', flavor: 'mocha', accent: 'mauve' }, '*')
```

- `flavor` is one of `latte`, `frappe`, `macchiato`, `mocha`.
- `accent` is one of the 14 Catppuccin accents (`rosewater` … `lavender`).
- Without a message (e.g. the file opened directly), follow `prefers-color-scheme`:
  **Latte** for light, **Mocha** for dark.
- In the listener, check `event.source === window.parent`, `data.type === 'gic-theme'`,
  and that the values are in the allowed lists.

## Starter template

`docs/examples/review-starter.html` is a complete, working starter. It already has:

- the CSS variables for all 4 flavors (`--ctp-base`, `--ctp-text`, `--ctp-mauve`, …),
  generated from the official `@catppuccin/palette` package,
- `--ctp-accent` with `[data-accent="…"]` overrides for all 14 accents,
- the `postMessage` theme listener with the `prefers-color-scheme` fallback,
- a responsive layout and a small interactive self-check.

Minimal skeleton (see the example file for the full palette):

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Subject — Interactive Review</title>
  <style>
    :root, [data-flavor="latte"] { --ctp-base: …; --ctp-text: …; --ctp-mauve: …; /* all 26 */ }
    [data-flavor="frappe"]       { /* … */ }
    [data-flavor="macchiato"]    { /* … */ }
    [data-flavor="mocha"]        { /* … */ }
    :root { --ctp-accent: var(--ctp-mauve); }
    [data-accent="blue"] { --ctp-accent: var(--ctp-blue); } /* … one per accent … */
    body { background: var(--ctp-base); color: var(--ctp-text); }
    h2 { color: var(--ctp-accent); }
  </style>
  <script>
    (function () {
      var FLAVORS = ['latte', 'frappe', 'macchiato', 'mocha'];
      var ACCENTS = ['rosewater','flamingo','pink','mauve','red','maroon','peach',
                     'yellow','green','teal','sky','sapphire','blue','lavender'];
      var root = document.documentElement, fromParent = false;
      var media = matchMedia('(prefers-color-scheme: dark)');
      function system() { if (!fromParent) root.dataset.flavor = media.matches ? 'mocha' : 'latte'; }
      system(); media.addEventListener('change', system);
      addEventListener('message', function (e) {
        if (e.source !== parent || !e.data || e.data.type !== 'gic-theme') return;
        if (FLAVORS.indexOf(e.data.flavor) >= 0) { fromParent = true; root.dataset.flavor = e.data.flavor; }
        if (ACCENTS.indexOf(e.data.accent) >= 0) root.dataset.accent = e.data.accent;
      });
    })();
  </script>
</head>
<body>
  <main><!-- your review --></main>
</body>
</html>
```

## How to upload

Uploading is a **manager** job (admins cannot upload reviews). Scripts and AI assistants use the
manager's **API key** with the `reviews:write` scope; see "Authenticating as an AI assistant or
script" in `docs/AI-HOW-ADD-QUIZ.md` for how to get one and the rules.

### Dashboard

**Dashboard → Interactive reviews → Upload review**: choose the subject, title, optional slug
(derived from the title if empty), order, *Published*, and the `.html` file.

### API

`POST /api/admin/reviews` (manager session, or API key with `reviews:write`), `multipart/form-data`:

| Field | Required | Meaning |
|---|---|---|
| `file` | yes | the `.html` file (`text/html`, UTF-8) |
| `subject_id` | yes | which subject it belongs to |
| `title` | yes | shown in the review list and header |
| `slug` | no | `lowercase-with-dashes`, unique per subject (default: from title); used in `?r=` |
| `sort_order` | no | ordering in the list (default 0, ascending) |
| `is_published` | no | `true`/`false` (default `false`); only published reviews are public |

```bash
export GIC_API=http://localhost:8080/api
export GIC_KEY=gic_…   # the manager's API key (reviews:write)

curl -H "Authorization: Bearer $GIC_KEY" \
  -F 'subject_id=1' -F 'title=HCI Review' -F 'slug=hci-review' \
  -F 'sort_order=0' -F 'is_published=true' \
  -F 'file=@docs/examples/review-starter.html;type=text/html' \
  $GIC_API/admin/reviews
```

`subject_id` comes from `GET $GIC_API/subjects` (public). List existing reviews, including
unpublished ones, with `GET $GIC_API/admin/reviews?subject_id=1`.

The subject's `has_review` flag (`GET /api/subjects/:slug`) becomes `true` once at least
one published review exists. Until then the home page shows "Coming soon".

## Replacing or updating a review without breaking its URL

`PUT /api/admin/reviews/:id` takes the same multipart fields, all optional:

- A new `file` **overwrites the same R2 key**, so the stored URL never changes.
- The API adds `?v=<updated_at>` to the URL it returns, so browsers and the CDN fetch the
  new version (objects are also served with `Cache-Control: public, max-age=60`).
- Changing `slug` changes the `?r=` link on the site; keep it if people bookmarked it.

```bash
curl -H "Authorization: Bearer $GIC_KEY" -X PUT -F 'file=@review-v2.html;type=text/html' \
  $GIC_API/admin/reviews/3
```

`DELETE /api/admin/reviews/:id` removes the row and the R2 object.
