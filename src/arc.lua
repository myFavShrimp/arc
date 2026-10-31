system "node-1" {
    address = "0.0.0.0",
    user = "root",
}

task "hello-world" {
    handler = function(system)
        log.info("Hello World!")
    end,
    tags = {"hello_world"},
}

task "hello-arc" {
    handler = function(system)
        log.info("Hello arc!")
    end,
    tags = {"hello_arc"},
    when = function()
        return task["hello-world"].state == "success"
    end,
}
