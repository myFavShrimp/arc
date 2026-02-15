local container = {}

function container.engine(system)
    if system.type == "local" then
        return "docker"
    else
        return "podman"
    end
end

function container.compose(system)
    if system.type == "local" then
        return "docker compose"
    else
        return "podman-compose"
    end
end

return container
