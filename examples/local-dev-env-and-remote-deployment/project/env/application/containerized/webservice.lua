local helpers = require("helpers")

local config = {
    image_name = "webservice",
    image_tag = "latest",
    port = 8080,
    tar_name = "webservice.tar",
}

tasks["ensure_webservice_directories"] = {
    targets = {"remote"},
    handler = function(system)
        local service_dir = helpers.paths.service_dir(system, "webservice")

        system:directory(service_dir):create()
    end,
    tags = {"directories"},
}

tasks["build_webservice_image"] = {
    targets = {"remote"},
    handler = function()
        local project_root = arc.project_root_path .. "/.."

        local build_result = host:run_command(
            "docker build -t " .. config.image_name .. ":" .. config.image_tag
            .. " " .. project_root
        )

        if build_result.exit_code ~= 0 then
            error("Could not build webservice image: \n" .. build_result.stdout .. "\n" .. build_result.stderr)
        end
    end,
    tags = {"build"},
}

tasks["export_webservice_image"] = {
    targets = {"remote"},
    requires = {"build_webservice_image"},
    handler = function()
        local project_root = arc.project_root_path .. "/.."

        host:run_command(
            "docker save " .. config.image_name .. ":" .. config.image_tag
            .. " -o " .. project_root .. "/" .. config.tar_name
        )
    end,
    tags = {"deploy"},
}

tasks["upload_webservice_image"] = {
    targets = {"remote"},
    requires = {"export_webservice_image", "cleanup_webservice_tar"},
    handler = function(system)
        local project_root = arc.project_root_path .. "/.."

        local remote_tar_path = "/tmp/" .. config.tar_name
        local local_tar_path = project_root .. "/" .. config.tar_name

        system:file(remote_tar_path).content = host:file(local_tar_path).content
    end,
    tags = {"deploy"},
}

tasks["load_webservice_image"] = {
    targets = {"remote"},
    handler = function(system)
        local remote_tar_path = "/tmp/" .. config.tar_name

        local load_result = system:run_command("podman load -i " .. remote_tar_path)
        if load_result.exit_code ~= 0 then
            error("Could not load webservice image: " .. load_result.stderr)
        end
    end,
    tags = {"deploy"},
}

tasks["cleanup_webservice_tar"] = {
    targets = {"remote"},
    handler = function(system)
        local project_root = arc.project_root_path .. "/.."

        local remote_result = system:run_command("rm -f /tmp/" .. config.tar_name)
        if remote_result.exit_code ~= 0 then
            error("Could not clean up remote tar: " .. remote_result.stderr)
        end

        local local_result = host:run_command("rm -f " .. project_root .. "/" .. config.tar_name)
        if local_result.exit_code ~= 0 then
            error("Could not clean up local tar: " .. local_result.stderr)
        end
    end,
    tags = {"deploy"},
}

tasks["setup_webservice_service"] = {
    targets = {"remote"},
    handler = function(system)
        local service_dir = helpers.paths.service_dir(system, "webservice")

        local compose_template = host:file("application/containerized/webservice/docker-compose.yml").content

        local compose_content = template.render(compose_template, {
            image = config.image_name,
            version = config.image_tag,
            port = config.port,
        })

        system:file(service_dir .. "docker-compose.yml").content = compose_content
    end,
    tags = {"setup"},
}

tasks["start_webservice_service"] = {
    targets = {"remote"},
    handler = function(system)
        local service_dir = helpers.paths.service_dir(system, "webservice")
        local compose = helpers.container.compose(system)

        local restart_result = system:run_command("cd " .. service_dir .. " && " .. compose .. " up -d --force-recreate")

        if restart_result.exit_code ~= 0 then
            error("Could not restart webservice: " .. restart_result.stderr)
        end
    end,
    tags = {"start"},
}
