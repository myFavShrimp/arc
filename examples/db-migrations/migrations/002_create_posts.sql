CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id),
    title VARCHAR(200) NOT NULL,
    body TEXT,
    created_at TIMESTAMP DEFAULT NOW()
);
