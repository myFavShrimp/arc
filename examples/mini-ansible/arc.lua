local playbook = format.yaml.decode(host:file("playbook.yaml").content)

-- Register systems
for name, system_definition in pairs(playbook.systems) do
    system(name, system_definition)
end

-- Register groups
if playbook.groups then
    for name, group_definition in pairs(playbook.groups) do
        group(name, group_definition)
    end
end

-- Playbook vars available in templates
local vars = playbook.vars or {}

-- Register tasks
for _, task_definition in ipairs(playbook.tasks) do
    local task_config = {}

    if task_definition.targets then
        task_config.targets = task_definition.targets
    end

    if task_definition.on_fail then
        task_config.on_fail = task_definition.on_fail
    end

    if task_definition.packages then
        task_config.handler = function(system)
            system:run_command("apt-get update")

            local pkgs = table.concat(task_definition.packages, " ")
            local result = system:run_command("apt-get install -y " .. pkgs)

            if result.exit_code ~= 0 then
                error("Failed to install packages: " .. result.stderr)
            end
        end
    elseif task_definition.copy then
        task_config.handler = function(system)
            local dest = system:file(task_definition.copy.dest)
            dest.content = host:file(task_definition.copy.src).content

            if task_definition.copy.permissions then
                dest.permissions = tonumber(task_definition.copy.permissions, 8)
            end
        end
    elseif task_definition.template then
        task_config.handler = function(system)
            local tmpl = host:file(task_definition.template.src).content

            local context = {}

            for k, v in pairs(vars) do
                context[k] = v
            end

            context.system_name = system.name
            context.system_address = system.address

            system:file(task_definition.template.dest).content = template.render(tmpl, context)
        end
    elseif task_definition.service then
        task_config.handler = function(system)
            local name = task_definition.service.name
            local state = task_definition.service.state

            local running = system:run_command("service " .. name .. " status").exit_code == 0

            if state == "started" and not running then
                local result = system:run_command("service " .. name .. " start")

                if result.exit_code ~= 0 then
                    error("Failed to start " .. name .. ": " .. result.stderr)
                end
            elseif state == "stopped" and running then
                local result = system:run_command("service " .. name .. " stop")

                if result.exit_code ~= 0 then
                    error("Failed to stop " .. name .. ": " .. result.stderr)
                end
            elseif state == "restarted" then
                local result = system:run_command("service " .. name .. " restart")

                if result.exit_code ~= 0 then
                    error("Failed to restart " .. name .. ": " .. result.stderr)
                end
            end
        end
    end

    task(task_definition.name, task_config)
end
