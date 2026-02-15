local string_helper = {}

---Strip leading and trailing whitespace
---@param str string
---@return string
function string_helper.strip(str)
    return (str:gsub("^%s+", ""):gsub("%s+$", ""))
end

return string_helper
