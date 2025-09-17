targets.systems["node-1"] = {
    address = "0.0.0.0",
    user = "root",
}

tasks["hello-world"] = {
    handler = function(system)
        log.info("Hello World!")
    end,
    tags = {"hello_world"},
    dependencies = {},
}

tasks["hello-arc"] = {
    handler = function(system)
        log.info("Hello arc!")
    end,
    tags = {"hello_arc"},
    dependencies = {"hello_world"},
}
