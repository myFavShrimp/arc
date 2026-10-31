system "server" {
    address = "127.0.0.1",
    port = 2222,
    user = "root",
}

system "local-dev" {
    type = "local",
}

group "remote" {
    members = {"server"},
}

group "dev" {
    members = {"local-dev"},
}

return nil
