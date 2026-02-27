local config = {
    image = "webservice",
    tag = "latest",
    tar = "webservice.tar",
    blue_port = 8081,
    green_port = 8082,
}

tasks["build_webservice"] = {
    handler = function()
        local project_root = arc.project_root_path .. "/.."

        local result = host:run_command(
            "docker build -t " .. config.image .. ":" .. config.tag
            .. " " .. project_root
        )

        if result.exit_code ~= 0 then
            error("Failed to build webservice:\n" .. result.stdout .. "\n" .. result.stderr)
        end
    end,
}

tasks["export_webservice"] = {
    requires = { "build_webservice" },
    handler = function()
        local project_root = arc.project_root_path .. "/.."

        local result = host:run_command(
            "docker save " .. config.image .. ":" .. config.tag
            .. " -o " .. project_root .. "/" .. config.tar
        )

        if result.exit_code ~= 0 then
            error("Failed to export webservice image: " .. result.stderr)
        end
    end,
}

tasks["upload_webservice"] = {
    requires = { "export_webservice", "cleanup_tar" },
    handler = function(system)
        local project_root = arc.project_root_path .. "/.."

        system:file("/tmp/" .. config.tar).content =
            host:file(project_root .. "/" .. config.tar).content
    end,
}

tasks["load_webservice"] = {
    requires = { "upload_webservice" },
    handler = function(system)
        local result = system:run_command("podman load -i /tmp/" .. config.tar)

        if result.exit_code ~= 0 then
            error("Failed to load webservice image: " .. result.stderr)
        end
    end,
}

tasks["cleanup_tar"] = {
    requires = { "load_webservice" },
    handler = function(system)
        local project_root = arc.project_root_path .. "/.."

        system:run_command("rm -f /tmp/" .. config.tar)
        host:run_command("rm -f " .. project_root .. "/" .. config.tar)
    end,
}

return config
