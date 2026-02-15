tasks["check_nginx_installed"] = {
    targets = { "backend" },
    handler = function(system)
        local result = system:run_command("which nginx")
        return result.exit_code == 0
    end,
}

tasks["install_nginx"] = {
    targets = { "backend" },
    requires = { "check_nginx_installed" },
    when = function()
        return tasks["check_nginx_installed"].result == false
    end,
    handler = function(system)
        system:run_command("apt-get update")

        local result = system:run_command("apt-get install -y nginx")

        if result.exit_code ~= 0 then
            error("Failed to install nginx: " .. result.stderr)
        end
    end,
}

tasks["deploy_backend_content"] = {
    targets = { "backend" },
    requires = { "install_nginx" },
    handler = function(system)
        local source = host:file("templates/index.html")
        local content = template.render(source.content, { system_name = system.name })

        local target = system:file("/var/www/html/index.html")
        target.content = content
    end,
}

tasks["check_nginx_running"] = {
    targets = { "backend" },
    requires = { "deploy_backend_content" },
    handler = function(system)
        local result = system:run_command("service nginx status")
        return result.exit_code == 0
    end,
}

tasks["enable_nginx"] = {
    targets = { "backend" },
    requires = { "check_nginx_running" },
    when = function()
        return tasks["check_nginx_running"].result == false
    end,
    handler = function(system)
        local result = system:run_command("service nginx start")

        if result.exit_code ~= 0 then
            error("Failed to start nginx: " .. result.stderr)
        end
    end,
}
