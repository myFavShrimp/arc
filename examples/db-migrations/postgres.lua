tasks["pg_config"] = {
    important = true,
    handler = function(system)
        -- This could build different configs per system. Kept simple for the example.
        return {
            host = "postgres",
            name = "app",
            user = "postgres",
        }
    end,
}

tasks["check_psql_installed"] = {
    handler = function(system)
        return system:run_command("which psql").exit_code == 0
    end,
}

tasks["install_psql"] = {
    requires = { "check_psql_installed" },
    when = function()
        return tasks["check_psql_installed"].result == false
    end,
    handler = function(system)
        system:run_command("apt-get update")

        local result = system:run_command("apt-get install -y postgresql-client")

        if result.exit_code ~= 0 then
            error("Failed to install postgresql-client: " .. result.stderr)
        end
    end,
}

tasks["wait_for_postgres"] = {
    requires = { "install_psql" },
    handler = function(system)
        local config = tasks["pg_config"].result

        for i = 1, 10 do
            local result = system:run_command(
                "psql -h " .. config.host .. " -U " .. config.user .. " -d " .. config.name
                .. ' -c "SELECT 1"'
            )

            if result.exit_code == 0 then
                return
            end

            log.warn("Waiting for PostgreSQL (attempt " .. i .. "/10)")
            system:run_command("sleep 1")
        end

        error("PostgreSQL not ready after 10 attempts")
    end,
}
