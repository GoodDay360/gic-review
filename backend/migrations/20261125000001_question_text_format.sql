-- How a question's text (prompt, options, answer, explanation) is displayed.
-- 'plain' keeps the text as typed; 'markdown' renders Markdown (code blocks, lists, …).
-- Existing questions stay 'plain' so they look exactly as before.
ALTER TABLE questions
    ADD COLUMN text_format VARCHAR(16) NOT NULL DEFAULT 'plain' AFTER multi_select;
