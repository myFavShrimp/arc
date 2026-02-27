targets.systems["local"] = {
    type = "local",
}

local src_dir = arc.project_root_path .. "/project/src"
local build_dir = arc.project_root_path .. "/project/build"

local sources = { "main", "greet", "bubble" }
local binary = "hello"

tasks["prepare_build_dir"] = {
    handler = function(system)
        system:directory(build_dir):create()
    end,
}

for _, name in ipairs(sources) do
    tasks["check_" .. name] = {
        requires = { "prepare_build_dir" },
        handler = function(system)
            local source = system:file(src_dir .. "/" .. name .. ".c")
            local object = system:file(build_dir .. "/" .. name .. ".o")

            if not object:exists() then
                return true
            end

            return source:metadata().modified > object:metadata().modified
        end,
    }

    tasks["compile_" .. name] = {
        requires = { "check_" .. name },
        when = function()
            return tasks["check_" .. name].result == true
        end,
        handler = function(system)
            local result = system:run_command(
                "gcc -c -I " .. src_dir
                .. " -o " .. build_dir .. "/" .. name .. ".o"
                .. " " .. src_dir .. "/" .. name .. ".c"
            )

            if result.exit_code ~= 0 then
                error("Failed to compile " .. name .. ".c: " .. result.stderr)
            end
        end,
    }
end

tasks["check_link"] = {
    requires = { "compile_main", "compile_greet", "compile_bubble" },
    handler = function(system)
        for _, name in ipairs(sources) do
            if tasks["compile_" .. name].state ~= "skipped" then
                return true
            end
        end

        return not system:file(build_dir .. "/" .. binary):exists()
    end,
}

tasks["link"] = {
    requires = { "check_link" },
    when = function()
        return tasks["check_link"].result == true
    end,
    handler = function(system)
        local objects = ""

        for _, name in ipairs(sources) do
            objects = objects .. " " .. build_dir .. "/" .. name .. ".o"
        end

        local result = system:run_command("gcc -o " .. build_dir .. "/" .. binary .. objects)

        if result.exit_code ~= 0 then
            error("Failed to link: " .. result.stderr)
        end
    end,
}
