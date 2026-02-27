local helpers = require("helpers")

local config = {
    image = "docker.io/grafana/grafana",
    version = "12.3.2",
    port = 3000,
    remote_port = 3100,
    admin_password = env.get("GRAFANA_ADMIN_PASSWORD") or error("GRAFANA_ADMIN_PASSWORD env variable is not set"),
}

tasks["ensure_grafana_directories"] = {
    handler = function(system)
        local service_dir = helpers.paths.service_dir(system, "grafana")
        local data_dir = helpers.paths.data_dir(system, "grafana")

        system:directory(service_dir):create()
        system:directory(data_dir):create()
        system:directory(service_dir .. "provisioning/datasources/"):create()
    end,
    tags = {"directories"},
}

tasks["setup_grafana_service"] = {
    handler = function(system)
        local id = tasks["obtain_user_id"].result
        local service_dir = helpers.paths.service_dir(system, "grafana")
        local data_dir = helpers.paths.data_dir(system, "grafana")

        local datasource_config = host:file("application/containerized/grafana/provisioning/datasources/tempo.yaml").content
        system:file(service_dir .. "provisioning/datasources/tempo.yaml").content = datasource_config

        local compose_template = host:file("application/containerized/grafana/docker-compose.yml").content

        local compose_content = template.render(compose_template, {
            image = config.image,
            version = config.version,
            user_id = id.user_id,
            group_id = id.group_id,
            port = system.type == "local" and config.port or config.remote_port,
            data_directory = data_dir,
            provisioning_directory = service_dir .. "provisioning/",
            admin_password = config.admin_password,
        })

        system:file(service_dir .. "docker-compose.yml").content = compose_content
    end,
    tags = {"setup"},
}

tasks["start_grafana_service"] = {
    handler = function(system)
        local service_dir = helpers.paths.service_dir(system, "grafana")
        local compose = helpers.container.compose(system)

        local restart_result = system:run_command("cd " .. service_dir .. " && " .. compose .. " up -d --force-recreate")

        if restart_result.exit_code ~= 0 then
            error("Could not restart grafana: " .. restart_result.stderr)
        end
    end,
    tags = {"start"},
}
