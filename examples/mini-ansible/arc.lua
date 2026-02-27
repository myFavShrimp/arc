local playbook = format.yaml.decode(host:file("playbook.yaml").content)

-- Register systems
for name, system in pairs(playbook.systems) do
    targets.systems[name] = system
end

-- Register groups
if playbook.groups then
    for name, group in pairs(playbook.groups) do
        targets.groups[name] = group
    end
end

-- Playbook vars available in templates
local vars = playbook.vars or {}

-- Register tasks
for _, task_def in ipairs(playbook.tasks) do
    local task = {}

    if task_def.targets then
        task.targets = task_def.targets
    end

    if task_def.on_fail then
        task.on_fail = task_def.on_fail
    end

    if task_def.packages then
        task.handler = function(system)
            system:run_command("apt-get update")

            local pkgs = table.concat(task_def.packages, " ")
            local result = system:run_command("apt-get install -y " .. pkgs)

            if result.exit_code ~= 0 then
                error("Failed to install packages: " .. result.stderr)
            end
        end
    elseif task_def.copy then
        task.handler = function(system)
            local dest = system:file(task_def.copy.dest)
            dest.content = host:file(task_def.copy.src).content

            if task_def.copy.permissions then
                dest.permissions = tonumber(task_def.copy.permissions, 8)
            end
        end
    elseif task_def.template then
        task.handler = function(system)
            local tmpl = host:file(task_def.template.src).content

            local context = {}

            for k, v in pairs(vars) do
                context[k] = v
            end

            context.system_name = system.name
            context.system_address = system.address

            system:file(task_def.template.dest).content = template.render(tmpl, context)
        end
    elseif task_def.service then
        task.handler = function(system)
            local name = task_def.service.name
            local state = task_def.service.state

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

    tasks[task_def.name] = task
end
