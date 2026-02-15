local helpers = require("helpers")

tasks["obtain_user_id"] = {
    handler = function(system)
        local user_id = helpers.string.strip(system:run_command("id -u").stdout)
        local group_id = helpers.string.strip(system:run_command("id -g").stdout)

        return {
            user_id = user_id,
            group_id = group_id,
        }
    end,
    important = true,
}
