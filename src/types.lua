---@meta

---@class CommandResult
---@field stdout string The command output
---@field stderr string The command error output  
---@field exit_code integer The command exit code

---@class FileMetadata
---@field path string Path to the file or directory
---@field size integer Size in bytes
---@field permissions integer Permission mode as number
---@field type "file"|"directory"|"unknown" Type of the item
---@field uid integer User ID of the owner
---@field gid integer Group ID of the owner
---@field accessed integer Last access time as Unix timestamp
---@field modified integer Last modification time as Unix timestamp

---@class File
---@field path string Path to the file (can be read and set, setting renames the file)
---@field content string Text content of the file (can be read and set)
---@field permissions integer File permissions (can be read and set as numeric mode)
local File = {}

---Get file metadata
---@return FileMetadata metadata File metadata information
function File:metadata() end

---Remove the file
function File:remove() end

---Check if file exists
---@return boolean exists True if file exists
function File:exists() end

---@class Directory
---@field path string Path to the directory (can be read and set, setting renames the directory)
---@field permissions integer Directory permissions (can be read and set as numeric mode)
local Directory = {}

---Create the directory
function Directory:create() end

---Remove the directory
function Directory:remove() end

---Get directory metadata
---@return FileMetadata metadata Directory metadata information
function Directory:metadata() end

---@class System
---@field name string The name of the system as defined in targets.systems
---@field address string The IP address of the system
---@field port integer The SSH port of the system
---@field user string The SSH user used to connect to the system
local System = {}

---Execute a command on the remote system
---@param cmd string The command to execute
---@return CommandResult result Command execution result
function System:run_command(cmd) end

---Get a File object representing a file on the remote system
---@param path string Path to the file
---@return File file File object
function System:file(path) end

---Get a Directory object representing a directory on the remote system
---@param path string Path to the directory
---@return Directory directory Directory object
function System:directory(path) end

---@class TaskDefinition
---@field handler fun(system: System): any The function that implements the task
---@field dependencies? string[] Array of task names that must be executed before this task
---@field tags? string[] Array of tags associated with the task, used for filtering
---@field groups? string[] Array of group names this task should run on
---@field result? any The result of the task execution (available after execution)

---@class SystemDefinition
---@field address string IP address or hostname of the system
---@field user string SSH username for the system
---@field port? integer SSH port (defaults to 22)

---@class GroupDefinition
---@field members string[] List of system names that belong to this group

---@class TargetsConfig
---@field systems table<string, SystemDefinition> Map of system names to system definitions
---@field groups table<string, GroupDefinition> Map of group names to group definitions

---Global tasks table for defining automation tasks
---@type table<string, TaskDefinition>
tasks = {}

---Global targets configuration for systems and groups
---@type TargetsConfig
targets = {
    systems = {},
    groups = {}
}

---Environment variables module
---@class EnvModule
local env = {}

---Get the value of an environment variable
---@param var_name string Name of the environment variable
---@return string|nil value Value of the environment variable or nil if not set
function env.get(var_name) end

---Local host operations module
---@class HostModule
local host = {}

---Execute a command on the local system
---@param cmd string The command to execute
---@return CommandResult result Command execution result
function host:run_command(cmd) end

---Get a File object representing a file on the local system
---@param path string Path to the file
---@return File file File object
function host:file(path) end

---Get a Directory object representing a directory on the local system
---@param path string Path to the directory
---@return Directory directory Directory object
function host:directory(path) end

---JSON formatting utilities
---@class FormatModule
local format = {}

---Convert a Lua value to JSON
---@param value any Value to convert
---@return string json JSON string representation
function format.to_json(value) end

---Convert a Lua value to pretty-printed JSON
---@param value any Value to convert
---@return string json Pretty-printed JSON string
function format.to_json_pretty(value) end

---Parse a JSON string to a Lua value
---@param json_string string JSON string to parse
---@return any value Parsed Lua value
function format.from_json(json_string) end

---Template rendering module
---@class TemplateModule
local template = {}

---Render a template with given context
---@param template_content string Template content
---@param context table Variables to use for template rendering
---@return string rendered Rendered template as string
function template.render(template_content, context) end

---Logging utilities
---@class LogModule
local log = {}

---Log a debug message
---@param message string Debug message to log
function log.debug(message) end

---Log an info message  
---@param message string Info message to log
function log.info(message) end

---Log a warning message
---@param message string Warning message to log
function log.warn(message) end

---Log an error message
---@param message string Error message to log
function log.error(message) end
