tasks["is_podman_installed"] = {
    handler = function(system)
        local result = system:run_command("which podman")
        return result.exit_code == 0
    end,
    targets = {"remote"},
}

tasks["install_podman"] = {
    when = function()
        return tasks["is_podman_installed"].result == false
    end,
    handler = function(system)
        local result = system:run_command("apt-get update && apt-get install -y podman podman-compose")

        if result.exit_code ~= 0 then
            error("Could not install podman: " .. result.stderr)
        end
    end,
    requires = {"is_podman_installed"},
    targets = {"remote"},
}
