# Arc

WIP

Run tasks on remote hosts.

```lua
-- arc.lua
targets.system["frontend-server"] = {
    address = "192.168.1.100",
    user = "root",
}

tasks["check nginx"] = {
    handler = function (system)
        local result = system:run_command("nginx -v")

        return result.exit_code == 0
    end,
    dependencies = {}
}

tasks["install nginx"] = {
    handler = function (system)
        local nginx_installed = tasks["check nginx"].result

        if nginx_installed ~= nil and not nginx_installed then
            return system:run_command("apt install nginx")
        end
    end,
    dependencies = {"check nginx"}
}

tasks["print nginx installation error"] = {
    handler = function (system)
        local installation_result = tasks["install nginx"].result

        if installation_result ~= nil and installation_result.exit_code ~= 0 then
            print(installation_result.stderr)
        end
    end,
    dependencies = {"install nginx"}
}
