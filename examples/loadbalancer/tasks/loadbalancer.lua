tasks["check_haproxy_installed"] = {
    targets = { "loadbalancer" },
    handler = function(system)
        local result = system:run_command("which haproxy")
        return result.exit_code == 0
    end,
}

tasks["install_haproxy"] = {
    targets = { "loadbalancer" },
    requires = { "check_haproxy_installed" },
    when = function()
        return tasks["check_haproxy_installed"].result == false
    end,
    handler = function(system)
        system:run_command("apt-get update")

        local result = system:run_command("apt-get install -y haproxy")

        if result.exit_code ~= 0 then
            error("Failed to install haproxy: " .. result.stderr)
        end
    end,
}

tasks["deploy_haproxy_config"] = {
    targets = { "loadbalancer" },
    requires = { "install_haproxy" },
    handler = function(system)
        local source = host:file("configs/haproxy.cfg")
        local target = system:file("/etc/haproxy/haproxy.cfg")

        target.content = source.content
    end,
}

tasks["check_haproxy_running"] = {
    targets = { "loadbalancer" },
    requires = { "deploy_haproxy_config" },
    handler = function(system)
        local result = system:run_command("service haproxy status")
        return result.exit_code == 0
    end,
}

tasks["enable_haproxy"] = {
    targets = { "loadbalancer" },
    requires = { "check_haproxy_running" },
    when = function()
        return tasks["check_haproxy_running"].result == false
    end,
    handler = function(system)
        local result = system:run_command("service haproxy start")

        if result.exit_code ~= 0 then
            error("Failed to start haproxy: " .. result.stderr)
        end
    end,
}
