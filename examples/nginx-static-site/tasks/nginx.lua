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

        local result = system:run_command("apt-get install -y nginx")

        if result.exit_code ~= 0 then
            error("Failed to install nginx: " .. result.stderr)
        end
    end,
}

tasks["check_nginx_running"] = {
    requires = { "install_nginx" },
    handler = function(system)
        local result = system:run_command("service nginx status")
        return result.exit_code == 0
    end,
}

tasks["enable_nginx"] = {
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
