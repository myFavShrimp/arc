targets.systems["web-server"] = {
    address = "127.0.0.1",
    port = 2222,
    user = "root",
}

require("tasks.nginx")
require("tasks.frontend")
