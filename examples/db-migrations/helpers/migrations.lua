local fs = require("helpers/fs")
local pg = require("helpers/postgres")

local migrations = {}

---@class Migration
---@field name string
---@field path string
---@field hash string

---Returns a map of applied migration versions to their hashes
---@param system RemoteSystem|LocalSystem
---@return table<string, string>
function migrations.get_applied(system)
    local output = pg.exec(system,
        "SELECT version || '|' || hash FROM schema_migrations ORDER BY version"
    )

    local applied = {}

    for line in output:gmatch("[^\n]+") do
        local version, h = line:match("^(.+)|(.+)$")
        applied[version] = h
    end

    return applied
end

---Returns a sorted list of local migration files with their hashes
---@return Migration[]
function migrations.get_local()
    local entries = host:directory("migrations"):entries()
    local result = {}

    for _, entry in ipairs(entries) do
        local meta = entry:metadata()

        if meta and meta.type == "file" then
            table.insert(result, {
                name = entry.file_name,
                path = entry.path,
                hash = fs.sha256(entry.path),
            })
        end
    end

    table.sort(result, function(a, b)
        return a.name < b.name
    end)

    return result
end

return migrations
