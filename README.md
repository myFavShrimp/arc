# Arc

WIP

Run tasks on remote hosts.

```lua
-- arc.lua
targets.systems["frontend-server"] = {
    address = "192.168.1.100",
    user = "root",
}

tasks["check nginx"] = {
    handler = function (system)
        local result = system:run_command("nginx -v")

        return result.exit_code == 0
    end,
    tags = {"setup nginx"}
}

tasks["install nginx"] = {
    handler = function (system)
        local nginx_installed = tasks["check nginx"].result

        if nginx_installed == false then
            return system:run_command("apt install nginx")
        end
    end,
    dependencies = {"check nginx"}
    tags = {"setup nginx"}
}

tasks["print nginx installation error"] = {
    handler = function (system)
        local installation_result = tasks["install nginx"].result

        print(installation_result.stderr)
    end,
    dependencies = {"install nginx"}
}
