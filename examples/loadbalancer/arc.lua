targets.systems["loadbalancer"] = {
    address = "127.0.0.1",
    port = 2220,
    user = "root",
}

targets.systems["backend-1"] = {
    address = "127.0.0.1",
    port = 2221,
    user = "root",
}

targets.systems["backend-2"] = {
    address = "127.0.0.1",
    port = 2222,
    user = "root",
}

targets.groups["backend"] = {
    members = { "backend-1", "backend-2" }
}

require("tasks.loadbalancer")
require("tasks.backend")
