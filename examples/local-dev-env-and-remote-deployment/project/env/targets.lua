targets.systems["server"] = {
    address = "127.0.0.1",
    port = 2222,
    user = "root",
}

targets.systems["local-dev"] = {
    type = "local",
}

targets.groups["remote"] = {
    members = {"server"},
}

targets.groups["dev"] = {
    members = {"local-dev"},
}
