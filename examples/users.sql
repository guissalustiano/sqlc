PREPARE find_user AS SELECT id, name FROM users WHERE id = $1;
PREPARE list_users AS SELECT id, name FROM users;
PREPARE create_user AS INSERT INTO users(name) VALUES ($1);
PREPARE update_user AS UPDATE users SET name = $2 WHERE id = $1;
PREPARE delete_user AS DELETE FROM users WHERE id = $1;
