-- Images uploaded without a quiz now live under `images/` instead of `tmp/`.
-- The R2 objects themselves were moved separately (dev and prod share the bucket).
UPDATE quizzes SET cover_image_key = CONCAT('images/', SUBSTRING(cover_image_key, 5))
    WHERE cover_image_key LIKE 'tmp/%';
UPDATE questions SET image_key = CONCAT('images/', SUBSTRING(image_key, 5))
    WHERE image_key LIKE 'tmp/%';
UPDATE questions SET answer_image_key = CONCAT('images/', SUBSTRING(answer_image_key, 5))
    WHERE answer_image_key LIKE 'tmp/%';
UPDATE questions SET explanation_image_key = CONCAT('images/', SUBSTRING(explanation_image_key, 5))
    WHERE explanation_image_key LIKE 'tmp/%';
UPDATE question_options SET image_key = CONCAT('images/', SUBSTRING(image_key, 5))
    WHERE image_key LIKE 'tmp/%';
