local pg = require("helpers/postgres")
local migrations = require("helpers/migrations")

tasks["ensure_migrations_table"] = {
    requires = { "wait_for_postgres" },
    handler = function(system)
        pg.exec(system,
            "CREATE TABLE IF NOT EXISTS schema_migrations "
            .. "(version TEXT PRIMARY KEY, hash TEXT NOT NULL, applied_at TIMESTAMP DEFAULT NOW())"
        )
    end,
}

local changed_migrations_separator = "\n  - "

tasks["check_modified_migrations"] = {
    requires = { "ensure_migrations_table" },
    handler = function(system)
        local applied_migrations = migrations.get_applied(system)
        local local_migrations = migrations.get_local()

        local changed = {}

        for _, migration in ipairs(local_migrations) do
            local applied_hash = applied_migrations[migration.name]

            if applied_hash and applied_hash ~= migration.hash then
                table.insert(changed, migration.name)
            end
        end

        if #changed > 0 then
            error("Applied migrations have been modified:" .. changed_migrations_separator .. table.concat(changed, changed_migrations_separator))
        end
    end,
}

tasks["check_pending_migrations"] = {
    requires = { "ensure_migrations_table" },
    handler = function(system)
        local applied_migrations = migrations.get_applied(system)
        local local_migrations = migrations.get_local()

        local pending = {}

        for _, migration in ipairs(local_migrations) do
            if not applied_migrations[migration.name] then
                table.insert(pending, migration)
            end
        end

        if #pending > 0 then
            log.info(#pending .. " pending migration(s)")
        end

        return pending
    end,
}

tasks["apply_migrations"] = {
    requires = { "check_pending_migrations", "check_modified_migrations" },
    when = function()
        return #tasks["check_pending_migrations"].result > 0
    end,
    handler = function(system)
        local pending_migrations = tasks["check_pending_migrations"].result

        for _, migration in ipairs(pending_migrations) do
            log.info("Applying " .. migration.name)

            local sql = host:file(migration.path).content
            local wrapped = "BEGIN;\n"
                .. sql .. ";\n"
                .. "INSERT INTO schema_migrations (version, hash) VALUES ('"
                .. migration.name .. "', '" .. migration.hash .. "');\n"
                .. "COMMIT;\n"

            system:file("/tmp/migration.sql").content = wrapped
            pg.exec_file(system, "/tmp/migration.sql")
        end

        system:run_command("rm -f /tmp/migration.sql")

        log.info("Applied " .. #pending_migrations .. " migration(s)")
    end,
}
