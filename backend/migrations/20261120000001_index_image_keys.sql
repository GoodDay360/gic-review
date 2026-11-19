-- The orphan cleanup checks R2 keys against the database 50 at a time, so every
-- column that holds an R2 key needs an index for those lookups to stay cheap.
CREATE INDEX idx_quizzes_cover_image_key ON quizzes (cover_image_key);
CREATE INDEX idx_questions_image_key ON questions (image_key);
CREATE INDEX idx_questions_answer_image_key ON questions (answer_image_key);
CREATE INDEX idx_questions_explanation_image_key ON questions (explanation_image_key);
CREATE INDEX idx_options_image_key ON question_options (image_key);
CREATE INDEX idx_reviews_r2_key ON interactive_reviews (r2_key);
