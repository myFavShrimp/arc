local paths = {}

local function project_root()
    return arc.root_path .. "/.."
end

function paths.service_dir(system, service_name)
    if system.type == "local" then
        return project_root() .. "/.dev/services/" .. service_name .. "/"
    else
        return "/opt/services/" .. service_name .. "/"
    end
end

function paths.data_dir(system, service_name)
    if system.type == "local" then
        return project_root() .. "/.dev/data/" .. service_name .. "/"
    else
        return "/opt/data/" .. service_name .. "/"
    end
end

return paths
