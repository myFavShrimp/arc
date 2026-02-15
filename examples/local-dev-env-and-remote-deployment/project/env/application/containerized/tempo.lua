local helpers = require("helpers")

local config = {
    image = "docker.io/grafana/tempo",
    version = "2.10.0",
    otlp_http_port = 4318,
    remote_otlp_http_port = 4418,
    api_port = 3200,
    remote_api_port = 3300,
}

tasks["ensure_tempo_directories"] = {
    handler = function(system)
        local service_dir = helpers.paths.service_dir(system, "tempo")
        local data_dir = helpers.paths.data_dir(system, "tempo")

        system:directory(service_dir):create()
        system:directory(data_dir):create()
    end,
    tags = {"directories"},
}

tasks["setup_tempo_service"] = {
    handler = function(system)
        local id = tasks["obtain_user_id"].result
        local service_dir = helpers.paths.service_dir(system, "tempo")
        local data_dir = helpers.paths.data_dir(system, "tempo")

        local tempo_config = host:file("application/containerized/tempo/tempo.yaml").content
        system:file(service_dir .. "tempo.yaml").content = tempo_config

        local compose_template = host:file("application/containerized/tempo/docker-compose.yml").content

        local compose_content = template.render(compose_template, {
            image = config.image,
            version = config.version,
            user_id = id.user_id,
            group_id = id.group_id,
            otlp_http_port = system.type == "local" and config.otlp_http_port or config.remote_otlp_http_port,
            api_port = system.type == "local" and config.api_port or config.remote_api_port,
            data_directory = data_dir,
            config_file = service_dir .. "tempo.yaml",
        })

        system:file(service_dir .. "docker-compose.yml").content = compose_content
    end,
    tags = {"setup"},
}

tasks["start_tempo_service"] = {
    handler = function(system)
        local service_dir = helpers.paths.service_dir(system, "tempo")

        local engine = helpers.container.engine(system)
        local compose = helpers.container.compose(system)

        local result = system:run_command(engine .. " ps --format '{{.Names}}' | grep tempo")
        local is_running = result.exit_code == 0

        if not is_running then
            local up_result = system:run_command("cd " .. service_dir .. " && " .. compose .. " up -d")
            if up_result.exit_code ~= 0 then
                error("Could not start tempo: " .. up_result.stderr)
            end
        else
            local restart_result = system:run_command("cd " .. service_dir .. " && " .. compose .. " up -d --force-recreate")
            if restart_result.exit_code ~= 0 then
                error("Could not restart tempo: " .. restart_result.stderr)
            end
        end
    end,
    tags = {"start"},
}

return config
