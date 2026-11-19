-- A category, or one subject, can be shared with every manager at once (in addition to, or
-- instead of, sharing it with people by email). Everyone with a manager/admin account then gets
-- the same rights as an individual share: add content there and view, edit and publish it.
-- Only the category's owner (or an admin) can turn it on or off.
ALTER TABLE categories ADD COLUMN shared_with_all BOOLEAN NOT NULL DEFAULT FALSE AFTER created_by;
ALTER TABLE subjects ADD COLUMN shared_with_all BOOLEAN NOT NULL DEFAULT FALSE AFTER sort_order;
