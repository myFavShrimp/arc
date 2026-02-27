tasks["check_nginx_installed"] = {
    handler = function(system)
        local result = system:run_command("which nginx")
        return result.exit_code == 0
    end,
}

tasks["install_nginx"] = {
    requires = { "check_nginx_installed" },
    when = function()
        return tasks["check_nginx_installed"].result == false
    end,
    handler = function(system)
        system:run_command("apt-get update")

        local result = system:run_command("apt-get install -y nginx curl")

        if result.exit_code ~= 0 then
            error("Failed to install nginx: " .. result.stderr)
        end
    end,
}

tasks["deploy_nginx_config"] = {
    requires = { "install_nginx" },
    handler = function(system)
        system:file("/etc/nginx/sites-available/default").content =
            host:file("configs/app.conf").content

        -- Initialize upstream config to blue port if it doesn't exist yet
        local upstream = system:file("/etc/nginx/app-upstream.conf")

        if not upstream:exists() then
            upstream.content = template.render(
                host:file("templates/upstream.conf").content,
                { port = 8081 }
            )
        end
    end,
}

tasks["ensure_nginx_running"] = {
    requires = { "deploy_nginx_config" },
    handler = function(system)
        local result = system:run_command("service nginx status")

        if result.exit_code ~= 0 then
            local start_result = system:run_command("service nginx start")

            if start_result.exit_code ~= 0 then
                error("Failed to start nginx: " .. start_result.stderr)
            end
        else
            system:run_command("service nginx reload")
        end
    end,
}
