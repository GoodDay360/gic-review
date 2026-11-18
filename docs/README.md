# Let an AI create quizzes and interactive reviews for you

You can give an AI assistant (ChatGPT, Claude, Gemini, …) your course material, for example a few
PDF files, and let it create **quizzes** and **interactive reviews** in GIC Review for you,
through the API.

## What you need

| What | Where to get it |
|---|---|
| **API base URL** | `https://gic-review.onrender.com/api` |
| **API key** | Dashboard → **API key** → *Create API key*. Tick `quizzes:write` and/or `reviews:write`, and pick a short expiry. It is shown only once, so copy it. |
| **Year** and **subject** | Dashboard → **Years** / **Subjects**. They must already exist (only humans can create them), and they must be yours or shared with you. Use their *slug* (the short name in the link, e.g. `i4` and `hci`). |
| **The two guides** | [`AI-HOW-ADD-QUIZ.md`](AI-HOW-ADD-QUIZ.md) and [`AI-HOW-ADD-INTERACTIVE-REVIEW.md`](AI-HOW-ADD-INTERACTIVE-REVIEW.md) in this folder. They tell the AI exactly which API calls to make. |

### Giving the guides to the AI

The AI must be able to read the two guides. Either:

- **Attach them** to the chat as files, together with your PDFs (works everywhere), or
- **Link them**, if this repository is public, using the *raw* file links:
  - `https://raw.githubusercontent.com/GoodDay360/gic-review/main/docs/AI-HOW-ADD-QUIZ.md`
  - `https://raw.githubusercontent.com/GoodDay360/gic-review/main/docs/AI-HOW-ADD-INTERACTIVE-REVIEW.md`

  A link to a **private** repository won't work: the AI can't open it.

## Prompt template

Attach your PDFs (and the two guides), then send:

```text
API base URL: https://gic-review.onrender.com/api
API key: <your-api-key>
Year: <year-slug>        (e.g. i4)
Subject: <subject-slug>  (e.g. hci)

Read the attached PDF files and use them to create a quiz and an interactive review in
GIC Review. 

Follow the attached instructions from:
https://github.com/GoodDay360/gic-review/blob/main/docs/AI-HOW-ADD-INTERACTIVE-REVIEW.md
https://github.com/GoodDay360/gic-review/blob/main/docs/AI-HOW-ADD-QUIZ.md


- Write the quiz and the review in English.
- Keep everything unpublished (draft). I will review it and publish it myself.
- When you are done, tell me the quiz and review IDs and titles.
```

Change the last three lines as you like. For example, ask for one quiz per chapter, a
specific number of questions, another language, or let it publish directly.

Tip: the server can take up to a minute to wake up after being idle (free hosting). If the AI
reports a timeout on its first request, tell it to simply try again.

## After the AI is done

1. Open the dashboard: the new quiz is under **Quizzes**, the review under **Interactive
   reviews**, both as drafts.
2. Check them with **Preview** / *Play*, fix anything you don't like, then click **Publish**.

## Keep your API key safe

- Anyone with the key can add and edit content in your years and subjects. Only paste it into
  AI tools you trust.
- Give the key only the permissions the job needs and a short expiry.
- When you're done, or if the key leaks, **revoke** or **regenerate** it in Dashboard → API key.
  The old key stops working immediately.
