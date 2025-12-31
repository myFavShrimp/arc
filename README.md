# Arc

Arc (Automatic Remote Controller) is an infrastructure automation tool that uses Lua for scripting. It executes tasks on remote systems via SSH with a flexible API for managing configurations, files, and commands across multiple servers.

## Installation

1. Install [Rust](https://rust-lang.org/)

2. Install Arc using Cargo:

```bash
cargo install arc-automation
```

This will compile and install the `arc` binary to the Cargo bin directory (usually `~/.cargo/bin/`).

## Quick Start

### Creating a New Project

Initialize a new Arc project with type definitions for LSP support:

```bash
arc init /path/to/project
```

This command creates the project structure, type definitions for code completion and type checking, and a basic `arc.lua` file with example tasks.

### Minimal Example

```lua
-- Define a target system
targets.systems["web-server"] = {
    address = "192.168.1.100",
    user = "root",
}

-- Define a simple task
tasks["hello"] = {
    handler = function (system)
        local result = system:run_command("echo 'Hello from ' $(hostname)")
        print(result.stdout)
    end
}
```

Run the task:

```bash
arc run -t hello
```

See [myfavshrimp/cfg](https://github.com/myfavshrimp/cfg) for an example managing local development machine configurations.

## Core Concepts

### Targets

Targets define the systems where tasks will be executed. There are two types: individual systems and groups.

#### Systems

Systems can be either remote (accessed via SSH) or local (running on the Arc host machine).

##### Remote Systems

Remote systems represent individual servers with SSH connection details.

```lua
targets.systems["frontend-server"] = {
    address = "192.168.1.100",
    user = "root",
    port = 22,  -- optional, defaults to 22
}

targets.systems["api-server"] = {
    address = "192.168.1.101",
    user = "deploy",
    port = 2222,
}
```

##### Local Systems

Local systems execute tasks on the machine where Arc is running.

```lua
targets.systems["localhost"] = {
    type = "local"
}
```

Local systems use the same API as remote systems (same methods for `run_command()`, `file()`, `directory()`, etc.), but operations execute locally instead of over SSH. The `address`, `port`, and `user` properties return `nil` for local systems.

#### Groups

Groups organize multiple systems.

```lua
targets.groups["web-servers"] = {
    members = {"frontend-server", "api-server"}
}

targets.groups["prod"] = {
    members = {"prod-web-1", "prod-db-1"}
}
```

### Tasks

Tasks define operations to execute on target systems. Tasks execute in definition order on each system.

```lua
tasks["install_nginx"] = {
    handler = function(system)
        local result = system:run_command("apt install nginx -y")
        if result.exit_code ~= 0 then
            error("Failed to install nginx: " .. result.stderr)
        end
    end,
    tags = {"nginx", "setup"},
}

tasks["configure_nginx"] = {
    dependencies = {"install_nginx"},
    handler = function(system)
        local config = system:file("/etc/nginx/nginx.conf")
        config.content = "..."
    end,
    tags = {"nginx"},
}
```

See [Tasks API](#tasks-1) for all available fields.

## CLI Reference

### `arc init`

Initialize a new Arc project with type definitions for LSP support, code completion and type checking.

```bash
arc init /path/to/project
```

### `arc run`

Executes Arc tasks defined in the `arc.lua` file.

```
Usage: arc run [OPTIONS]

Options:
  -t, --tag <TAG>      Select tasks by tag
  -g, --group <GROUP>  Run tasks only on specific groups
  -d, --dry-run        Print tasks that would be executed without running them
      --no-deps        Skip dependency resolution and only run explicitly selected tasks
  -a, --all-tags       Run all tasks
  -h, --help           Print help
```

## Lua API Reference

Arc uses a restricted Lua environment. The following standard library modules are available:

- [Modules](https://www.lua.org/manual/5.1/manual.html#5.3) (`require`)
- [String Manipulation](https://www.lua.org/manual/5.1/manual.html#5.4) (`string.format`, `string.match`, `string.gsub`, etc.)
- [Table Manipulation](https://www.lua.org/manual/5.1/manual.html#5.5) (`table.insert`, `table.remove`, `table.sort`, etc.)
- [Mathemetical Functions](https://www.lua.org/manual/5.1/manual.html#5.6) (`math.floor`, `math.random`, etc.)

Not available: `io`, `os`, `debug`, `coroutine`. Use the provided Arc APIs (`system:run_command()`, `system:file()`, `env.get()`, etc.) instead.

### Tasks

Tasks are defined by assigning to the global `tasks` table. Tasks execute in definition order on each system.

#### Properties

- `handler`: Function that implements the task logic
  - *Parameters*: `system` - The system object to operate on
  - *Returns*: Optional result value accessible via `tasks["name"].result`

- `tags` (optional): Array of tags for filtering tasks. Tasks are automatically tagged with their name and source file path components (e.g., `modules/web/nginx.lua` adds tags: `modules`, `web`, `nginx`).

- `groups` (optional): Array of group names where this task should run. If omitted, runs on all groups.

- `dependencies` (optional): Array of tags this task depends on. Tasks with matching tags are included when this task is selected. Resolved transitively.

- `when` (optional): Guard predicate that determines if the task should run
  - *Returns*: `boolean` - If `false`, task is skipped

- `on_fail` (optional): Behavior when this task fails
  - `"continue"` (default): Proceed to next task
  - `"skip_system"`: Skip remaining tasks for this system
  - `"abort"`: Halt execution entirely

- `important` (optional): If `true`, always runs regardless of tag filters, `--no-deps`, and `skip_system`

#### State (read-only, available after execution)

- `result`: Return value from handler (nil if failed/skipped)
- `state`: `"success"`, `"failed"`, or `"skipped"`
- `error`: Error message if failed (nil otherwise)

Example:

```lua
tasks["check_nginx"] = {
    handler = function(system)
        return system:run_command("which nginx").exit_code == 0
    end
}

tasks["install_nginx"] = {
    dependencies = {"check_nginx"},
    when = function()
        return tasks["check_nginx"].result == false
    end,
    handler = function(system)
        local result = system:run_command("apt install nginx -y")
        if result.exit_code ~= 0 then
            error("Failed: " .. result.stderr)
        end
    end,
    on_fail = "skip_system",
}
```

Dependencies affect **which** tasks run, not **when**. Tasks always execute in definition order. If a task depends on something defined later, the dependency runs *after* the dependent task.

### System Object

The `system` object represents a connection to a target system (remote or local) and is passed to task handlers.

#### Properties

- `name`: The name of the system as defined in `targets.systems`
- `type`: The type of system - `"remote"` or `"local"`
- `address`: The IP address of the system (nil for local systems)
- `port`: The SSH port of the system (nil for local systems)
- `user`: The SSH user used to connect to the system (nil for local systems)

#### Methods

- `run_command(cmd)`: Execute a command on the system
  - *Parameters*: `cmd` (string) - The command to execute
  - *Returns*: A table with `stdout`, `stderr`, and `exit_code`

- `file(path)`: Get a File object representing a file on the system
  - *Parameters*: `path` (string) - Path to the file
  - *Returns*: A File object

- `directory(path)`: Get a Directory object representing a directory on the system
  - *Parameters*: `path` (string) - Path to the directory
  - *Returns*: A Directory object

Example:

```lua
tasks["check_service"] = {
    handler = function(system)
        log.info("Checking service on " .. system.name .. " at " .. system.address)
        local result = system:run_command("systemctl status nginx")
        return result.exit_code == 0
    end
}
```

### File Object

The File object represents a file on a target system and provides access to file content, metadata, and operations.

#### Properties

- `path`: Path to the file (can be read and set; setting the path renames the file)
- `file_name`: The name of the file without the directory path
- `content`: Text content of the file (can be read and set)
- `permissions`: File permissions (can be read and set as numeric mode)

#### Methods

- `metadata()`: Get file metadata
  - *Returns*: A table with file metadata (see Metadata Structure section)

- `remove()`: Remove the file

- `parent()`: Get the parent directory
  - *Returns*: A Directory object representing the parent directory

- `directory()`: Get the directory containing this file
  - *Returns*: A Directory object

Example:

```lua
tasks["configure_nginx"] = {
    handler = function(system)
        -- Create and write to a file
        local config_file = system:file("/etc/nginx/sites-available/default")
        config_file.content = "server {\n    listen 80 default_server;\n    root /var/www/html;\n}"

        -- Set permissions
        config_file.permissions = tonumber("644", 8)

        -- Get metadata
        local metadata = config_file:metadata()
        log.info("File size: " .. metadata.size .. " bytes")
    end
}

tasks["manage_file"] = {
    handler = function(system)
        -- Create, move and then delete a file
        local file = system:file("/path/to/file.txt")
        file.content = "New content"                 -- Write to file
        file.permissions = tonumber("755", 8)        -- Set permissions
        file.path = "/new-path/to/renamed-file.txt"  -- Rename file
        file:remove()                                -- Delete file
    end
}
```

### Directory Object

The Directory object represents a directory on a target system and provides access to directory operations and contents.

#### Properties

- `path`: Path to the directory (can be read and set; setting the path renames the directory)
- `file_name`: The name of the directory without the parent path
- `permissions`: Directory permissions (can be read and set as numeric mode)
- `entries`: Array of File and Directory objects representing the directory contents

#### Methods

- `create()`: Create the directory
- `remove()`: Remove the directory
- `metadata()`: Get directory metadata
  - *Returns*: A table with directory metadata (see Metadata Structure section)
- `parent()`: Get the parent directory
  - *Returns*: A Directory object representing the parent directory
- `directory()`: Get the directory itself
  - *Returns*: A Directory object (returns self)

Example:

```lua
tasks["setup_directory"] = {
    handler = function(system)
        -- Create directory structure
        local app_dir = system:directory("/var/www/myapp")
        app_dir:create()

        -- Set permissions
        app_dir.permissions = tonumber("755", 8)
    end
}

tasks["list_configs"] = {
    handler = function(system)
        -- Iterate through directory contents
        local dir = system:directory("/etc/nginx/sites-available")
        for _, entry in ipairs(dir:entries()) do
            -- Each entry is either a File or Directory object
            print(entry.path)
            print("Permissions: " .. entry.permissions)

            local metadata = entry:metadata()
            if metadata.type == "file" then
                print("File size: " .. metadata.size .. " bytes")
            elseif metadata.type == "directory" then
                print("Directory")
            end
        end
    end
}

tasks["manage_directory"] = {
    handler = function(system)
        -- Create, move and then delete a directory
        local dir = system:directory("/path/to/dir")
        dir:create()                                 -- Create directory
        dir.path = "/path/to/renamed-dir"            -- Rename directory
        dir:remove()                                 -- Delete directory
    end
}
```

### Metadata Structure

The `metadata()` method on File or Directory objects returns a table with the following fields:

- `path`: Path to the file or directory
- `size`: Size in bytes (number)
- `permissions`: Permission mode (number)
- `type`: Type of the item ("file", "directory", or "unknown")
- `uid`: User ID of the owner (number)
- `gid`: Group ID of the owner (number)
- `accessed`: Last access time as a Unix timestamp (number)
- `modified`: Last modification time as a Unix timestamp (number)

Example:

```lua
tasks["check_metadata"] = {
    handler = function(system)
        local file = system:file("/etc/hostname")
        local metadata = file:metadata()

        log.info("File size: " .. metadata.size)
        log.info("File type: " .. metadata.type)
        log.info("File permissions: " .. metadata.permissions)
        log.info("Last modified: " .. metadata.modified)
    end
}
```

### Environment Variables (env)

The `env` module provides access to environment variables. Arc automatically loads variables from `.env` files in the project directory.

#### Methods

- `get(var_name)`: Get the value of an environment variable
  - *Parameters*: `var_name` (string) - Name of the environment variable
  - *Returns*: Value of the environment variable (string) or nil if not set

Example:

```lua
tasks["deploy_app"] = {
    handler = function(system)
        local app_version = env.get("APP_VERSION") or "latest"
        local deploy_path = env.get("DEPLOY_PATH") or "/var/www"

        system:run_command("docker pull myapp:" .. app_version)
        system:run_command("docker run -d -v " .. deploy_path .. ":/app myapp:" .. app_version)
    end
}
```

### Host Module

The `host` module provides functions for interacting with the local system where Arc is running. It has the same interface as the `system` object but operates on the local machine and it's working directory is the one arc was executed in.

#### Methods

- `run_command(cmd)`: Execute a command on the local system
  - *Parameters*: `cmd` (string) - The command to execute
  - *Returns*: A table with `stdout`, `stderr`, and `exit_code`

- `file(path)`: Get a File object representing a file on the local system
  - *Parameters*: `path` (string) - Path to the file
  - *Returns*: A File object

- `directory(path)`: Get a Directory object representing a directory on the local system
  - *Parameters*: `path` (string) - Path to the directory
  - *Returns*: A Directory object

Example:

```lua
tasks["deploy_from_local"] = {
    handler = function(system)
        -- Read a local template
        local template = host:file("templates/nginx.conf").content

        -- Write to remote system
        local remote_config = system:file("/etc/nginx/nginx.conf")
        remote_config.content = template

        -- Restart service
        system:run_command("systemctl restart nginx")
    end
}

tasks["backup_to_local"] = {
    handler = function(system)
        -- Read from remote
        local remote_config = system:file("/etc/app/config.json").content

        -- Save locally
        local backup_dir = host:directory("backups/" .. system.name)
        backup_dir:create()

        local backup_file = host:file("backups/" .. system.name .. "/config.json")
        backup_file.content = remote_config
    end
}
```

### Format Module

The `format` module provides utilities for working with JSON data.

#### Functions

- `to_json(value)`: Convert a Lua value to JSON
  - *Parameters*: `value` (any) - Value to convert
  - *Returns*: JSON string

- `to_json_pretty(value)`: Convert a Lua value to pretty-printed JSON
  - *Parameters*: `value` (any) - Value to convert
  - *Returns*: JSON string

- `from_json(json_string)`: Parse a JSON string to a Lua value
  - *Parameters*: `json_string` (string) - JSON string to parse
  - *Returns*: Parsed Lua value

Example:

```lua
tasks["manage_json_config"] = {
    handler = function(system)
        -- Read a JSON configuration file
        local config_file = system:file("/etc/myapp/config.json")
        local config = format.from_json(config_file.content)

        -- Modify configuration
        config.debug = true
        config.log_level = "info"

        -- Write back to the file
        config_file.content = format.to_json_pretty(config)
    end
}

tasks["update_api_config"] = {
    handler = function(system)
        -- Get current config from an API
        local result = system:run_command("curl -s http://localhost:8080/api/config")
        local api_config = format.from_json(result.stdout)

        -- Update configuration
        api_config.settings.cache_ttl = 3600

        -- Send updated config back to API
        local json_config = format.to_json(api_config)
        system:run_command('curl -X POST -H "Content-Type: application/json" -d \'' .. json_config .. '\' http://localhost:8080/api/config')
    end
}
```

### Template Module

The `template` module provides template rendering capabilities using the Tera template engine.

#### Methods

- `render(template_content, context)`: Render a template with given context
  - *Parameters*:
    - `template_content` (string) - Template content
    - `context` (table) - Variables to use for template rendering
  - *Returns*: Rendered template as string

Example:

```lua
tasks["configure_web_server"] = {
    handler = function(system)
        -- Load a template from a local file
        local template_content = host:file("templates/nginx.conf.template").content

        -- Define context variables
        local context = {
            worker_processes = 4,
            worker_connections = 1024,
            server_name = system.name .. ".example.com",
            document_root = "/var/www/" .. system.name,
            environment = env.get("ENVIRONMENT") or "production"
        }

        -- Render and deploy configuration
        local config = template:render(template_content, context)
        system:file("/etc/nginx/nginx.conf").content = config

        -- Validate and reload
        local validation = system:run_command("nginx -t")
        if validation.exit_code == 0 then
            system:run_command("systemctl reload nginx")
            return true
        else
            error("Nginx configuration is invalid: " .. validation.stderr)
        end
    end
}
```

### Logging Module

The `log` module provides logging functions at various severity levels.

#### Functions

- `debug(value)`: Log a debug message
  - *Parameters*: `value` (any) - Value to log

- `info(value)`: Log an info message
  - *Parameters*: `value` (any) - Value to log

- `warn(value)`: Log a warning message
  - *Parameters*: `value` (any) - Value to log

- `error(value)`: Log an error message
  - *Parameters*: `value` (any) - Value to log

Example:

```lua
tasks["provision_database"] = {
    handler = function(system)
        log.info("Starting database provisioning on " .. system.name)

        local result = system:run_command("systemctl status postgresql")

        -- Log complex values like tables
        log.debug(result)

        if result.exit_code ~= 0 then
            log.warn("PostgreSQL not running, attempting to install")

            local install = system:run_command("apt-get install -y postgresql")
            if install.exit_code ~= 0 then
                log.error("Failed to install PostgreSQL: " .. install.stderr)
                return false
            end
        end

        log.debug("PostgreSQL installed and running")
        return true
    end
}
```

## LSP Support

Arc provides Language Server Protocol (LSP) support for Lua code editing with autocomplete, type checking, and inline documentation.

### Setup

1. Install the [Lua Language Server](https://github.com/LuaLS/lua-language-server) for the editor being used.

2. Initialize an Arc project to generate type definitions:

```bash
arc init /path/to/project
```

The `init` command creates `.luarc.json` and type definition files that enable the Lua Language Server to recognize Arc's API types.

## Contributing

Contributions are welcome! Please feel free to submit a pull request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
