local helpers = require("helpers")
local tempo_config = require("application/containerized/tempo")

tasks["setup_local_env"] = {
    targets = {"dev"},
    handler = function(system)
        local project_root = helpers.string.strip(host:run_command("pwd").stdout) .. "/.."

        local env_template = host:file("application/local/env").content

        local env_content = template.render(env_template, {
            otlp_port = tempo_config.otlp_http_port,
        })

        system:file(project_root .. "/.env").content = env_content
    end,
    tags = {"setup"},
}
