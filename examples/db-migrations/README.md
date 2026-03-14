# db-migrations

Runs SQL migrations against a PostgreSQL database. Tracks applied migrations in a `schema_migrations` table and only applies pending ones.

Migration files are discovered automatically from the `migrations/` directory and applied in alphabetical order.

## Requirements

- arc
- Docker

## Usage

Start the containers:

```bash
docker compose up -d
```

Run the migrations:

```bash
arc run --all-tags -s server
```

Run it again to see that already applied migrations are skipped.

To add a new migration, create a new SQL file in `migrations/` (e.g. `004_add_index.sql`) and run again.

## Cleanup

```bash
docker compose down
```
