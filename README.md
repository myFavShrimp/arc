# Arc

WIP

Run tasks on remote hosts.

```lua
-- init.lua
targets.add_system("frontend-server", {
    address = "192.168.1.100",
    user = "root",
})

targets.add(
    "check nginx",
    function (system)
        local result = operation.run_command("nginx -v")

        return result.exit_code == 0
    end
)

tasks.add(
    "install nginx",
    function (system)
        local nginx_installed = tasks.get_result("check nginx")

        if nginx_installed ~= nil and not nginx_installed then
            return operation.run_command("apt install nginx")
        end
    end
)

tasks.add(
    "print nginx installation error",
    function (system)
        local installation_result = tasks.get_result("install nginx")

        if installation_result ~= nil and installation_result.exit_code ~= 0 then
            print(installation_result.stderr)
        end
    end
)
```
