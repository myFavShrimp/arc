tasks["check_podman_installed"] = {
    handler = function(system)
        local result = system:run_command("which podman")
        return result.exit_code == 0
    end,
}

tasks["install_podman"] = {
    requires = { "check_podman_installed" },
    when = function()
        return tasks["check_podman_installed"].result == false
    end,
    handler = function(system)
        system:run_command("apt-get update")

        local result = system:run_command("apt-get install -y podman")

        if result.exit_code ~= 0 then
            error("Failed to install podman: " .. result.stderr)
        end
    end,
}
