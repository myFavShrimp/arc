local config = require("webservice")

tasks["determine_active_slot"] = {
    requires = { "ensure_nginx_running" },
    handler = function(system)
        local upstream = system:file("/etc/nginx/app-upstream.conf")
        local content = tostring(upstream.content)

        if string.find(content, tostring(config.blue_port)) then
            return {
                active = "blue",
                active_port = config.blue_port,
                inactive = "green",
                inactive_port = config.green_port,
            }
        else
            return {
                active = "green",
                active_port = config.green_port,
                inactive = "blue",
                inactive_port = config.blue_port,
            }
        end
    end,
}

tasks["deploy_to_inactive_slot"] = {
    requires = { "determine_active_slot", "load_webservice", "install_podman" },
    handler = function(system)
        local slots = tasks["determine_active_slot"].result
        local container_name = "webservice-" .. slots.inactive
        local port = slots.inactive_port

        log.info("Deploying to " .. slots.inactive .. " slot (port " .. port .. ")")

        -- Stop the inactive container if it exists
        system:run_command("podman rm -f " .. container_name)

        local result = system:run_command(
            "podman run -d"
            .. " --name " .. container_name
            .. " -p " .. port .. ":8080"
            .. " " .. config.image .. ":" .. config.tag
        )

        if result.exit_code ~= 0 then
            error("Failed to start " .. slots.inactive .. " container: " .. result.stderr)
        end
    end,
}

tasks["health_check"] = {
    requires = { "deploy_to_inactive_slot" },
    handler = function(system)
        local slots = tasks["determine_active_slot"].result

        log.info("Waiting for " .. slots.inactive .. " slot to become healthy...")

        system:run_command("sleep 2")

        local retries = 5

        for i = 1, retries do
            local result = system:run_command(
                "curl -sf http://127.0.0.1:" .. slots.inactive_port .. "/health"
            )

            if result.exit_code == 0 then
                log.info("Health check passed on attempt " .. i)
                return true
            end

            if i < retries then
                log.warn("Health check failed (attempt " .. i .. "/" .. retries .. "), retrying...")
                system:run_command("sleep 2")
            end
        end

        error("Health check failed after " .. retries .. " attempts - aborting switch")
    end,
}

tasks["switch_traffic"] = {
    requires = { "health_check" },
    handler = function(system)
        local slots = tasks["determine_active_slot"].result

        log.info(
            "Switching traffic from " .. slots.active
            .. " to " .. slots.inactive
            .. " (port " .. slots.inactive_port .. ")"
        )

        system:file("/etc/nginx/app-upstream.conf").content = template.render(
            host:file("templates/upstream.conf").content,
            { port = slots.inactive_port }
        )

        local result = system:run_command("nginx -t")

        if result.exit_code ~= 0 then
            error("Nginx config test failed: " .. result.stderr)
        end

        result = system:run_command("service nginx reload")

        if result.exit_code ~= 0 then
            error("Failed to reload nginx: " .. result.stderr)
        end

        log.info("Traffic switched to " .. slots.inactive .. " slot")
    end,
}

tasks["stop_old_slot"] = {
    requires = { "switch_traffic" },
    handler = function(system)
        local slots = tasks["determine_active_slot"].result
        local container_name = "webservice-" .. slots.active

        log.info("Stopping old " .. slots.active .. " slot")

        system:run_command("podman rm -f " .. container_name)
    end,
}
