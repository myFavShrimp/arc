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

To run tasks with specific tags:

```bash
arc --tags nginx
```

To run tasks on specific groups:

```bash
arc --groups web-servers
```

## API Reference

### System API

The `system` object is provided to task handlers and represents a connection to a remote system.

#### Properties

- `name`: The name of the system as defined in targets.systems
- `address`: The IP address of the system
- `port`: The SSH port of the system
- `user`: The SSH user used to connect to the system

#### Methods

- `run_command(cmd)`: Execute a command on the remote system
  - *Parameters*: `cmd` (string) - The command to execute
  - *Returns*: A table with the following fields:
    - `stdout`: Command standard output
    - `stderr`: Command standard error
    - `exit_code`: Command exit code

- `read_file(path)`: Read a file from the remote system
  - *Parameters*: `path` (string) - Path to the file
  - *Returns*: A table with the following fields:
    - `path`: Path to the file
    - `content`: Content of the file

- `write_file(path, content)`: Write a file to the remote system
  - *Parameters*:
    - `path` (string) - Path to the file
    - `content` (string) - Content to write
  - *Returns*: A table with the following fields:
    - `path`: Path to the file
    - `bytes_written`: Number of bytes written

- `rename_file(from, to)`: Rename a file on the remote system
  - *Parameters*:
    - `from` (string) - Original path
    - `to` (string) - New path

- `remove_file(path)`: Remove a file from the remote system
  - *Parameters*: `path` (string) - Path to the file

- `remove_directory(path)`: Remove a directory from the remote system
  - *Parameters*: `path` (string) - Path to the directory

- `create_directory(path)`: Create a directory on the remote system
  - *Parameters*: `path` (string) - Path to the directory

- `set_permissions(path, mode)`: Set permissions for a file or directory
  - *Parameters*:
    - `path` (string) - Path to the file or directory
    - `mode` (number) - Permissions mode (e.g. `tonumber("755", 8)`)

- `metadata(path)`: Get metadata for a file or directory
  - *Parameters*: `path` (string) - Path to the file or directory
  - *Returns*: A table with the following fields:
    - `path`: Path to the file
    - `size`: Size in bytes
    - `permissions`: Permission mode
    - `type`: Type of file ("file", "directory", or "unknown")
    - `uid`: User ID
    - `gid`: Group ID
    - `accessed`: Last access time
    - `modified`: Last modification time

### File System API

The `fs` object provides access to the local file system.

#### Methods

- `read_file(path)`: Read a file from the local file system
  - *Parameters*: `path` (string) - Path to the file
  - *Returns*: Content of the file as a string

### Templates API

The `template` object allows rendering templates using the Tera templating engine.

#### Methods

- `render(template_content, context)`: Render a template with given context
  - *Parameters*:
    - `template_content` (string) - Template content
    - `context` (table) - Variables to use for template rendering
  - *Returns*: Rendered template as string

### Format API

The `format` object provides utilities for working with JSON.

#### Methods

- `to_json(value)`: Convert a Lua value to JSON
  - *Parameters*: `value` (any) - Value to convert
  - *Returns*: JSON string

- `to_json_pretty(value)`: Convert a Lua value to pretty-printed JSON
  - *Parameters*: `value` (any) - Value to convert
  - *Returns*: Pretty-printed JSON string

- `from_json(json_string)`: Parse a JSON string to a Lua value
  - *Parameters*: `json_string` (string) - JSON string to parse
  - *Returns*: Parsed Lua value

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
local result = tasks["dependent_task"].result
```

### Target Definition

Targets define the systems and groups that tasks will run on.

#### Systems

```lua
targets.systems["system_name"] = {
    address = "ip_or_hostname",
    user = "ssh_username",
    port = 22, -- optional, defaults to 22
}
```

#### Groups

```lua
targets.groups["group_name"] = {
    members = {"system1", "system2"}, 
}
```
