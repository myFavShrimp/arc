# Arc

Arc is an infrastructure automation tool similar to Ansible, but using Lua for scripting. It allows you to define tasks that can be executed on remote systems via SSH.

## Installation

Clone the repository and build the project:

```bash
git clone https://github.com/myFavShrimp/arc.git
cd arc
cargo build --release
```

The binary will be available at `target/release/arc`.

## Usage

Arc uses a Lua script (`arc.lua` by default) to define targets and tasks.

### Basic Example

```lua
-- Define a target system
targets.systems["frontend-server"] = {
    address = "192.168.1.100",
    user = "root",
}

targets.systems["api-server"] = {
    address = "192.168.1.101",
    user = "root",
    port = 42,  -- defaults to 22 if not specified
}

-- Define a group of systems
targets.groups["web-servers"] = {
    members = {"frontend-server", "api-server"}
}

-- Define tasks
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

To run all tasks on all systems:

```bash
arc
```

To run tasks with a specific tag:

```bash
arc --tag nginx
```

To run tasks on specific a group:

```bash
arc --group web-servers
```

## CLI Reference

Arc provides several command-line options:

```
Usage:
    arc [OPTIONS]

Options:
  -v, --verbose...     Enable verbose output (repeat for increased verbosity)
  -t, --tag <TAG>      Filter tasks by tag
  -g, --group <GROUP>  Run tasks only on specific groups
  -d, --dry-run        Perform a dry run without executing commands or modifying the file system
  -h, --help           Print help information
```

### Examples

Run tasks with the "nginx" tag:
```bash
arc -t nginx
```

Run tasks on the "web-servers" group with verbose output:
```bash
arc -g web-servers -v
```

Run tasks with multiple tags and groups:
```bash
arc -t nginx -t security -g web-servers -g database-servers
```

Perform a dry run without executing commands:
```bash
arc --dry-run
```

## API Reference

### System Object

The `system` object represents a connection to a remote system and is provided to task handlers.

#### Properties

- `name`: The name of the system as defined in targets.systems
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

### File Object

The File object represents a file on a remote system.

#### Properties

- `path`: Path to the file (can be read and set, setting renames the file)
- `content`: Content of the file (can be read and set)
- `permissions`: File permissions (can be read and set as numeric mode)

#### Methods

- `metadata()`: Get file metadata
  - *Returns*: A table with file metadata (see Metadata Structure below)

- `remove()`: Remove the file

### Directory Object

The Directory object represents a directory on a remote system.

#### Properties

- `path`: Path to the directory (can be read and set, setting renames the directory)
- `permissions`: Directory permissions (can be read and set as numeric mode)

#### Methods

- `create()`: Create the directory
- `remove()`: Remove the directory
- `metadata()`: Get directory metadata

### Metadata Structure

When calling the `metadata()` method on File or Directory objects, a table with the following fields is returned:

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
local file = system:file("/etc/hostname")
local metadata = file:metadata()

print("File size: " .. metadata.size)
print("File type: " .. metadata.type)
print("File permissions: " .. metadata.permissions)
print("Last modified: " .. metadata.modified)
```

### Environment Variables (env)

The `env` module provides access to environment variables.

#### Methods

- `get(var_name)`: Get the value of an environment variable
  - *Parameters*: `var_name` (string) - Name of the environment variable
  - *Returns*: Value of the environment variable (string) or nil if not set

Example:

```lua
local home_dir = env.get("HOME")
local user = env.get("USER")

if home_dir then
    print("Home directory: " .. home_dir)
end
```

### Host Module

The `host` module provides functions for interacting with the local system (where Arc is running).

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
-- Execute a local command
local result = host:run_command("ls -la")
print("Command output: " .. result.stdout)

-- Work with a local file
local local_file = host:file("/tmp/example.txt")
local_file.content = "This is a local file"

-- Create a local directory
local local_dir = host:directory("/tmp/arc_test")
local_dir:create()
```

### Format API

The `format` object provides utilities for working with JSON.

#### Methods

- `to_json(value)`: Convert a Lua value to JSON
  - *Parameters*: `value` (any) - Value to convert
  - *Returns*: JSON string

- `from_json(json_string)`: Parse a JSON string to a Lua value
  - *Parameters*: `json_string` (string) - JSON string to parse
  - *Returns*: Parsed Lua value

### Templates API

The `template` object allows rendering templates.

#### Methods

- `render(template_content, context)`: Render a template with given context
  - *Parameters*:
    - `template_content` (string) - Template content
    - `context` (table) - Variables to use for template rendering
  - *Returns*: Rendered template as string

### Logging

Various logging functions are available globally:

- `debug(message)`: Log a debug message
- `info(message)`: Log an info message
- `warn(message)`: Log a warning message
- `error(message)`: Log an error message

### Task Definition

Tasks are defined using the `tasks` global table:

```lua
tasks["task_name"] = {
    handler = function(system)
        -- Task implementation
        return result
    end,
    dependencies = {"other_task"}, -- Optional
    tags = {"tag1", "tag2"}, -- Optional
    groups = {"group1", "group2"}, -- Optional
}
```

- `handler`: The function that implements the task. Takes a system object and returns a result.
- `dependencies`: Array of task names that must be executed before this task.
- `tags`: Array of tags associated with the task, used for filtering.
- `groups`: Array of group names this task should run on. If not specified, the task runs on all groups.

Within a task, you can access the result of a previously executed dependency using:

```lua
local dependency_result = tasks["dependency_task_name"].result
```

## Working with Files and Directories

Arc provides a comprehensive file system API for managing files and directories on remote systems:

```lua
-- Working with files
local file = system:file("/path/to/file.txt")
file.content = "New content"                 -- Write to file
file.permissions = tonumber("755", 8)        -- Set permissions
file.path = "/path/to/renamed-file.txt"      -- Rename file
file:remove()                                -- Delete file

-- Working with directories
local dir = system:directory("/path/to/dir")
dir:create()                                 -- Create directory
dir.permissions = tonumber("755", 8)         -- Set permissions
dir.path = "/path/to/renamed-dir"            -- Rename directory
dir:remove()                                 -- Delete directory

-- Getting metadata
local metadata = file:metadata()
print("File size: " .. metadata.size)
print("File type: " .. metadata.type)
print("File permissions: " .. metadata.permissions)
print("Last modified: " .. metadata.modified)
```

## Examples

For more usage examples, refer to the included sample scripts or visit the project documentation.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
