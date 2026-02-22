# arc

arc (Automatic Remote Controller) is an automation tool that uses Lua for scripting. It executes tasks on the local or remote systems via SSH with a flexible API for managing configurations, files, and commands across multiple servers.

[![asciicast](https://asciinema.org/a/L5fJR5JOLVziZyvA.svg)](https://asciinema.org/a/L5fJR5JOLVziZyvA)

## Installation

1. Install [Rust](https://rust-lang.org/)

2. Install arc using Cargo:

```bash
cargo install arc-automation
```

This will compile and install the `arc` binary to the Cargo bin directory (usually `~/.cargo/bin/`).

Please make sure you have the required dependencies installed:

**Fedora**

```bash
sudo dnf group install development-tools
sudo dnf install openssl-devel
```

**Ubuntu / Debian**

```bash
sudo apt install build-essential libssl-dev
```

**MacOS**

- Install [Homebrew](https://brew.sh/)

```bash
brew install openssl
```

## Quick Start

### Creating a New Project

Initialize a new arc project with type definitions for LSP support:

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
    handler = function(system)
        local result = system:run_command("echo 'Hello from ' $(hostname)")
        print(result.stdout)
    end
}
```

Run the task:

```bash
arc run -s web-server -t hello
```

See the [examples](examples/) directory for more complete usage examples. You can also explore the available commands using `arc --help`, `arc run --help` etc.

## Core Concepts

### Targets

Targets define the systems where tasks will be executed. There are two types: individual systems and groups.

#### Systems

Systems can be either remote (accessed via SSH) or local (running on the arc host machine).

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

Authentication is handled through the SSH agent. The host's public key must be present in the remote server's `authorized_keys`.

##### Local Systems

Local systems execute tasks on the machine where arc is running.

```lua
targets.systems["localhost"] = {
    type = "local"
}
```

Local systems use the same API as remote systems but operations execute locally instead of over SSH. The `address`, `port`, and `user` properties return `nil` for local systems.

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
    requires = {"install_nginx"},
    handler = function(system)
        local config = system:file("/etc/nginx/nginx.conf")
        config.content = "..."
    end,
    tags = {"nginx"},
}
```

See [Tasks API](#tasks-1) for all available fields.

## Lua API Reference

arc uses a restricted [LuaJIT](https://luajit.org/) environment. The following standard library modules are available:

- [Modules](https://www.lua.org/manual/5.1/manual.html#5.3) (`require`)
- [String Manipulation](https://www.lua.org/manual/5.1/manual.html#5.4) (`string.format`, `string.match`, `string.gsub`, etc.)
- [Table Manipulation](https://www.lua.org/manual/5.1/manual.html#5.5) (`table.insert`, `table.remove`, `table.sort`, etc.)
- [Mathematical Functions](https://www.lua.org/manual/5.1/manual.html#5.6) (`math.floor`, `math.random`, etc.)

Not available: `io`, `os`, `debug`, `coroutine`. Use the provided arc APIs (`system:run_command()`, `system:file()`, `env.get()`, etc.) instead.

The global `print()` function is an alias for `log.info()`.

### Tasks

Tasks are defined by assigning to the global `tasks` table. Tasks execute in definition order on each system.

#### Properties

- `handler`: Function that implements the task logic
  - *Parameters*: `system` - The system object to operate on
  - *Returns*: Optional result value accessible via `tasks["name"].result`

- `tags` (optional): Array of tags for filtering tasks. Tasks are automatically tagged with their name and source file path components (e.g., `modules/web/nginx.lua` adds tags: `modules`, `web`, `nginx`).

- `targets` (optional): Array of group or system names where this task should run. If omitted, runs on all systems.

- `requires` (optional): Array of tags this task requires. Tasks with matching tags are included when this task is selected. Resolved transitively.

- `when` (optional): Guard predicate that determines if the task should run
  - *Returns*: `boolean` - If `false`, task is skipped

- `on_fail` (optional): Behavior when this task fails
  - `"continue"`: Proceed to next task
  - `"skip_system"`: Skip remaining tasks for this system
  - `"abort"` (default): Halt execution entirely

- `important` (optional): If `true`, always runs regardless of tag filters, `--no-reqs`, and `skip_system`

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
    requires = {"check_nginx"},
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

Requires affect **which** tasks run, not **when**. Tasks always execute in definition order. If a task requires something defined later, the required task runs *after* the requiring task.

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
        log.info("Checking nginx on " .. system.name)
        local result = system:run_command("systemctl status nginx")
        return result.exit_code == 0
    end
}
```

### File Object

The File object represents a file on a target system and provides access to file content, metadata, and operations.

#### Properties

- `path`: Path to the file (can be read and set; setting the path moves the file)
- `file_name`: The name of the file without the directory path (can be read and set)
- `content`: File content handle (can be read and set). Reading returns a `FileContent` object that acts as a lazy reference. Assigning a `FileContent` from one file to another transfers the data. The handle converts to a string automatically when used with `tostring()`, `..`, `print()`, or `template.render()`.
- `permissions`: File permissions (can be read and set as numeric mode; returns `nil` if file doesn't exist)

#### Methods

- `exists()`: Check if file exists
  - *Returns*: `boolean` - `true` if file exists, `false` otherwise

- `metadata()`: Get file metadata
  - *Returns*: A table with file metadata (see [Metadata Structure](#metadata-structure)), or `nil` if file doesn't exist

- `remove()`: Remove the file

- `directory()`: Get the directory containing this file
  - *Returns*: A Directory object, or `nil` if at root path

Example:

```lua
tasks["configure_nginx"] = {
    handler = function(system)
        local config_file = system:file("/etc/nginx/sites-available/default")

        config_file.content = "server {\n    listen 80 default_server;\n    root /var/www/html;\n}"
        config_file.permissions = tonumber("644", 8)

        local metadata = config_file:metadata()

        if metadata and metadata.size then
            log.info("File size: " .. metadata.size .. " bytes")
        end
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
- `file_name`: The name of the directory without the parent path (can be read and set)
- `permissions`: Directory permissions (can be read and set as numeric mode; returns `nil` if directory doesn't exist)

#### Methods

- `create()`: Create the directory (including any missing ancestor directories)
- `remove()`: Remove the directory
- `exists()`: Check if directory exists
  - *Returns*: `boolean` - `true` if directory exists, `false` otherwise
- `metadata()`: Get directory metadata
  - *Returns*: A table with directory metadata (see [Metadata Structure](#metadata-structure)), or `nil` if directory doesn't exist
- `parent()`: Get the parent directory
  - *Returns*: A Directory object representing the parent directory, or `nil` if at root path
- `entries()`: Get directory entries
  - *Returns*: Array of File and Directory objects representing the directory contents

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
            local metadata = entry:metadata()

            if metadata then
                print(entry.path .. " (" .. metadata.type .. ")")

                if metadata.type == "file" and metadata.size then
                    print("  Size: " .. metadata.size .. " bytes")
                end
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

The `metadata()` method on File or Directory objects returns a table with the following fields, or `nil` if the file/directory doesn't exist:

- `path`: Path to the file or directory
- `size`: Size in bytes (number, or `nil` if unavailable)
- `permissions`: Permission mode (number, or `nil` if unavailable)
- `type`: Type of the item ("file", "directory", or "unknown")
- `uid`: User ID of the owner (number, or `nil`; **always `nil` on local systems**)
- `gid`: Group ID of the owner (number, or `nil`; **always `nil` on local systems**)
- `accessed`: Last access time as a Unix timestamp (number, or `nil` if unavailable)
- `modified`: Last modification time as a Unix timestamp (number, or `nil` if unavailable)

Example:

```lua
tasks["check_metadata"] = {
    handler = function(system)
        local file = system:file("/etc/hostname")
        local metadata = file:metadata()

        if metadata then
            log.info("File size: " .. (metadata.size or "unknown"))
            log.info("File type: " .. metadata.type)
            log.info("File permissions: " .. (metadata.permissions or "unknown"))
            log.info("Last modified: " .. (metadata.modified or "unknown"))
        else
            log.info("File does not exist")
        end
    end
}
```

### Environment Variables (env)

The `env` module provides access to environment variables. arc automatically loads variables from `.env` files in the project directory. Variables defined in the `.env` file take precedence over already defined ones.

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

### Host Object

The global `host` object provides functions for interacting with the local system where arc is running. It has the same interface as the `system` object but operates on the local machine and its working directory is the directory where `arc.lua` is located.

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
        -- Streams the file directly from local to remote without loading it into memory
        system:file("/etc/nginx/nginx.conf").content = host:file("templates/nginx.conf").content

        -- Restart service
        system:run_command("systemctl restart nginx")
    end
}

tasks["backup_to_local"] = {
    handler = function(system)
        -- Streams from remote to local
        local backup_dir = host:directory("backups/" .. system.name)
        backup_dir:create()

        host:file("backups/" .. system.name .. "/config.json").content =
            system:file("/etc/app/config.json").content
    end
}
```

### Format Module

The `format` module provides serialization and deserialization utilities for various data formats. Each format is accessible as a sub-object with `encode` and `decode` methods.

#### `format.json`

- `encode(value)`: Encode a Lua value as JSON
- `encode_pretty(value)`: Encode a Lua value as pretty-printed JSON
- `decode(input)`: Decode a JSON string to a Lua value

#### `format.toml`

- `encode(value)`: Encode a Lua value as TOML
- `decode(input)`: Decode a TOML string to a Lua value

#### `format.yaml`

- `encode(value)`: Encode a Lua value as YAML
- `decode(input)`: Decode a YAML string to a Lua value

#### `format.url`

- `encode(value)`: Encode a Lua value as a URL query string (`x-www-form-urlencoded`)
- `decode(input)`: Decode a URL query string to a Lua value

#### `format.env`

- `encode(value)`: Encode a table as dotenv `KEY=VALUE` lines
- `decode(input)`: Decode dotenv `KEY=VALUE` lines to a table

Example:

```lua
tasks["manage_json_config"] = {
    handler = function(system)
        local config_file = system:file("/etc/myapp/config.json")
        local config = format.json.decode(tostring(config_file.content))

        config.debug = true
        config.log_level = "info"

        config_file.content = format.json.encode_pretty(config)
    end
}

tasks["manage_toml_config"] = {
    handler = function(system)
        local config_file = system:file("/etc/myapp/config.toml")
        local config = format.toml.decode(tostring(config_file.content))

        config.log_level = "info"

        config_file.content = format.toml.encode(config)
    end
}
```

### Template Module

The `template` module provides template rendering capabilities using the [Tera](https://keats.github.io/tera/docs/#templates) template engine.

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
        local config = template.render(template_content, context)
        system:file("/etc/nginx/nginx.conf").content = config

        -- Validate and reload
        local validation = system:run_command("nginx -t")

        if validation.exit_code == 0 then
            system:run_command("systemctl reload nginx")
        else
            error("Nginx configuration is invalid: " .. validation.stderr)
        end
    end
}
```

### Arc Module

The global `arc` object provides information about the arc project and environment.

#### Properties

- `project_root_path`: The absolute path to the project root directory (where `arc.lua` is located)
- `home_path`: The absolute path to the user's home directory

Example:

```lua
tasks["deploy_config"] = {
    handler = function(system)
        -- Read a file relative to the project root
        local config = host:file(arc.project_root_path .. "/configs/app.json").content
        system:file("/etc/myapp/config.json").content = config
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
        log.info("Provisioning database on " .. system.name)

        local check = system:run_command("which psql")
        log.debug("which psql exit code: " .. check.exit_code)

        if check.exit_code ~= 0 then
            log.warn("PostgreSQL not found, attempting to install")

            local install_result = system:run_command("apt-get install -y postgresql")
            if install_result.exit_code ~= 0 then
                error("Failed to install PostgreSQL: " .. install_result.stderr)
            end
        end

        log.info("PostgreSQL is available")
    end
}
```

## LSP Support

arc provides Language Server Protocol (LSP) support for Lua code editing with autocomplete, type checking, and inline documentation.

### Setup

1. Install the [Lua Language Server](https://github.com/LuaLS/lua-language-server) for the editor being used.

2. Initialize an arc project to generate type definitions:

```bash
arc init /path/to/project
```

The `init` command creates `.luarc.json` and type definition files that enable the Lua Language Server to recognize arc's API types.

## Contributing

Contributions are welcome! Please feel free to submit any issue or pull request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
