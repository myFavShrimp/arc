local fs = {}

---@param path string
---@return string
function fs.sha256(path)
    local result = host:run_command("sha256sum " .. path)

    if result.exit_code ~= 0 then
        error("Failed to hash " .. path .. ": " .. result.stderr)
    end

    return result.stdout:match("^(%S+)")
end

return fs
