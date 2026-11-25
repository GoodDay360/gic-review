-- Images an interactive review's HTML embeds (uploaded through /admin/uploads/image).
-- Their keys only live inside the review's HTML file in R2, so without this table the
-- orphan cleanup saw them as unreferenced and deleted them.
CREATE TABLE review_images (
    review_id BIGINT       NOT NULL,
    image_key VARCHAR(255) NOT NULL,
    PRIMARY KEY (review_id, image_key),
    INDEX idx_review_images_image_key (image_key),
    CONSTRAINT fk_review_images_review FOREIGN KEY (review_id)
        REFERENCES interactive_reviews (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

-- FALSE until the review's HTML has been scanned for image keys. Existing reviews start
-- unscanned; the orphan cleanup scans them (reading their HTML from R2) before it deletes
-- anything.
ALTER TABLE interactive_reviews
    ADD COLUMN images_indexed BOOLEAN NOT NULL DEFAULT FALSE;
