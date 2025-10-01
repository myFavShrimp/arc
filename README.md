# Arc

Arc (Automatic Remote Controller) is an infrastructure automation tool that uses Lua for scripting. It executes tasks on remote systems via SSH with a flexible API for managing configurations, files, and commands across multiple servers.

## Installation

1. Install Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Clone the repository and navigate to the project directory:

```bash
git clone https://github.com/myFavShrimp/arc.git
cd arc
```

3. Install Arc using Cargo:

```bash
cargo install --path .
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
arc run
```

## Core Concepts

### Targets

Targets define the remote systems where tasks will be executed. There are two types: individual systems and groups.

#### Systems

Systems represent individual servers with SSH connection details:

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

#### Groups

Groups organize multiple systems for batch operations:

```lua
targets.groups["web-servers"] = {
    members = {"frontend-server", "api-server"}
}

targets.groups["production"] = {
    members = {"web-servers", "database-servers"}  -- can include other groups
}
```

### Tasks

Tasks define operations to execute on remote systems. Each task consists of a handler function and optional metadata.

#### Basic Task Structure

```lua
tasks["task_name"] = {
    handler = function(system)
        -- Task implementation
        return result
    end,
    dependencies = {"other_task"},  -- optional
    tags = {"tag1", "tag2"},        -- optional
    groups = {"group1"},            -- optional
}
```

- `handler`: Function that implements the task logic. Receives a system object and returns a result.
- `dependencies`: Array of task names that must execute before this task. Results from dependencies can be accessed via `tasks["dependency_name"].result`.
- `tags`: Array of tags for filtering tasks. Tasks are automatically tagged with the task name and path components when defined in separate files (e.g., `modules/web/nginx.lua` adds tags: `modules`, `web`, `nginx`).
- `groups`: Array of group names where this task should run. If omitted, the task runs on all groups.

#### Task Dependencies

Dependencies ensure tasks execute in the correct order. Dependency results can be accessed within dependent tasks:

```lua
tasks["check_nginx"] = {
    handler = function (system)
        local result = system:run_command("nginx -v")
        return result.exit_code == 0
    end,
    tags = {"nginx"}
}

tasks["install_nginx"] = {
    handler = function (system)
        local nginx_installed = tasks["check_nginx"].result

        if nginx_installed == false then
            return system:run_command("apt install nginx -y")
        end
    end,
    dependencies = {"check_nginx"},
    tags = {"nginx"}
}
```

### Running Tasks

Execute tasks using the `arc run` command with optional filters:

```bash
# Run all tasks on all systems
arc run

# Run tasks with specific tag
arc run --tag nginx

# Run tasks on specific group
arc run --group web-servers

# Combine multiple filters
arc run -t nginx -t security -g web-servers

# Perform a dry run without executing commands
arc run --dry-run
```

## CLI Reference

### `arc init`

Initialize a new Arc project with type definitions for LSP support, code completion and type checking.

```bash
arc init /path/to/project
```

### `arc run`

Execute Arc tasks defined in the `arc.lua` file.

```
Usage: arc run [OPTIONS]

Options:
  -t, --tag <TAG>      Filter tasks by tag
  -g, --group <GROUP>  Run tasks only on specific groups
  -d, --dry-run        Perform a dry run without executing commands or modifying the file system
  -h, --help           Print help
```

## Lua API Reference

### System Object

The `system` object represents a connection to a remote system and is passed to task handlers.

#### Properties

- `name`: The name of the system as defined in `targets.systems`
- `address`: The IP address of the system
- `port`: The SSH port of the system
- `user`: The SSH user used to connect to the system

#### Methods

- `run_command(cmd)`: Execute a command on the remote system
  - *Parameters*: `cmd` (string) - The command to execute
  - *Returns*: A table with `stdout`, `stderr`, and `exit_code`

- `file(path)`: Get a File object representing a file on the remote system
  - *Parameters*: `path` (string) - Path to the file
  - *Returns*: A File object

- `directory(path)`: Get a Directory object representing a directory on the remote system
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

The File object represents a file on a remote system and provides access to file content, metadata, and operations.

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

The Directory object represents a directory on a remote system and provides access to directory operations and contents.

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

The `host` module provides functions for interacting with the local system where Arc is running. It has the same interface as the `system` object but operates on the local machine.

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

## Contributing

Contributions are welcome! Please feel free to submit a pull request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
