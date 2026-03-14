---@class PgConfig
---@field host string
---@field name string
---@field user string

local pg = {}

---@param system RemoteSystem|LocalSystem
---@param sql string
---@return string
function pg.exec(system, sql)
    local config = tasks["pg_config"].result

    local result = system:run_command(
        "psql -h " .. config.host .. " -U " .. config.user .. " -d " .. config.name
        .. ' -t -A -c "' .. sql .. '"'
    )

    if result.exit_code ~= 0 then
        error("SQL failed: " .. result.stderr)
    end

    return result.stdout
end

---@param system RemoteSystem|LocalSystem
---@param path string
function pg.exec_file(system, path)
    local config = tasks["pg_config"].result

    local result = system:run_command(
        "psql -h " .. config.host .. " -U " .. config.user .. " -d " .. config.name
        .. " -f " .. path
    )

    if result.exit_code ~= 0 then
        error("SQL file failed: " .. result.stderr)
    end
end

return pg
