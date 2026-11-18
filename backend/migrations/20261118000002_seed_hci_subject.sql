-- Seed exactly ONE subject. Other subjects are added through the admin panel.
INSERT INTO subjects (slug, name, description, color, sort_order)
VALUES ('hci', 'Human Computer Interaction',
        'Usability, user-centred design, interaction principles and evaluation methods.',
        'mauve', 0);
