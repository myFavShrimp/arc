targets.systems["node-1"] = {
    address = "0.0.0.0",
    user = "root",
}

tasks["hello-world"] = {
    handler = function(system)
        log.info("Hello World!")
    end,
    tags = {"hello_world"},
}

tasks["hello-arc"] = {
    handler = function(system)
        log.info("Hello arc!")
    end,
    tags = {"hello_arc"},
    when = function()
        return tasks["hello-world"].state == "success"
    end,
}
