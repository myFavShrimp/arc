tasks["deploy_html"] = {
    requires = { "install_nginx" },
    handler = function(system)
        local source = host:file("frontend/index.html")
        local target = system:file("/var/www/html/index.html")

        target.content = source.content
    end,
}

tasks["deploy_css"] = {
    requires = { "install_nginx" },
    handler = function(system)
        local source = host:file("frontend/style.css")
        local target = system:file("/var/www/html/style.css")

        target.content = source.content
    end,
}
