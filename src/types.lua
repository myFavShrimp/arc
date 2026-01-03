---@meta _
---Type definitions for Arc

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
---@field file_name string The name of the file without the directory path
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

---Get the directory containing this file
---@return Directory directory Directory object
function File:directory() end


---@class Directory
---@field path string Path to the directory (can be read and set, setting renames the directory)
---@field file_name string The name of the directory without the parent path
---@field permissions integer Directory permissions (can be read and set as numeric mode)
local Directory = {}

---Create the directory
function Directory:create() end

---Remove the directory
function Directory:remove() end

---Get directory metadata
---@return FileMetadata metadata Directory metadata information
function Directory:metadata() end

---Get directory entries
---@return (File|Directory)[] entries Array of File and Directory objects representing the directory contents
function Directory:entries() end

---Get the parent directory
---@return Directory parent Parent directory object
function Directory:parent() end


---@class RemoteSystem
---@field name string The name of the system as defined in targets.systems
---@field type "remote" The type of system
---@field address string The IP address of the system
---@field port integer The SSH port of the system
---@field user string The SSH user used to connect to the system
local RemoteSystem = {}

---Execute a command on the remote system
---@param cmd string The command to execute
---@return CommandResult result Command execution result
function RemoteSystem:run_command(cmd) end

---Get a File object representing a file on the remote system
---@param path string Path to the file
---@return File file File object
function RemoteSystem:file(path) end

---Get a Directory object representing a directory on the remote system
---@param path string Path to the directory
---@return Directory directory Directory object
function RemoteSystem:directory(path) end


---@class LocalSystem
---@field name string The name of the system as defined in targets.systems
---@field type "local" The type of system
---@field address nil The IP address of the system
---@field port nil The SSH port of the system
---@field user nil The SSH user used to connect to the system
local LocalSystem = {}

---Execute a command on the local system
---@param cmd string The command to execute
---@return CommandResult result Command execution result
function LocalSystem:run_command(cmd) end

---Get a File object representing a file on the local system
---@param path string Path to the file
---@return File file File object
function LocalSystem:file(path) end

---Get a Directory object representing a directory on the local system
---@param path string Path to the directory
---@return Directory directory Directory object
function LocalSystem:directory(path) end


---Behavior when a task fails
---@alias FailureBehavior "continue" | "skip_system" | "abort"

---Task execution state
---@alias TaskState "success" | "failed" | "skipped"

---@class TaskDefinition
---@field handler fun(system: RemoteSystem|LocalSystem): any The function that implements the task
---@field when? fun(): boolean Guard predicate to determine if task should run (check previous task states/results)
---@field on_fail? FailureBehavior Behavior when this task fails (default: "continue")
---@field tags? string[] Array of tags associated with the task, used for filtering
---@field groups? string[] Array of group names this task should run on
---@field dependencies? string[] Array of tags that this task depends on
---@field important? boolean If true, task always runs regardless of tag filters, --no-deps, and skip_system
---@field result? any The result of the task execution (available after execution)
---@field state? TaskState State of the task after execution
---@field error? string Error message if the task failed

---@class RemoteSystemDefinition
---@field type? "remote" System type (optional, defaults to "remote")
---@field address string IP address or hostname of the system
---@field user string SSH username for the system
---@field port? integer SSH port (defaults to 22)

---@class LocalSystemDefinition
---@field type "local" System type - must be "local" to target the local system

---@alias SystemDefinition RemoteSystemDefinition|LocalSystemDefinition

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
local Env = {}

---Get the value of an environment variable
---@param var_name string Name of the environment variable
---@return string|nil value Value of the environment variable or nil if not set
function Env.get(var_name) end

---Environment variables module
---@type EnvModule
env = {}

--- Global host class for accessing the host system
---@class Host
local Host = {}

---Execute a command on the local system
---@param cmd string The command to execute
---@return CommandResult result Command execution result
function Host:run_command(cmd) end

---Get a File object representing a file on the local system
---@param path string Path to the file
---@return File file File object
function Host:file(path) end

---Get a Directory object representing a directory on the local system
---@param path string Path to the directory
---@return Directory directory Directory object
function Host:directory(path) end

--- Global host instance for accessing the host system
---@type Host
host = {}


---JSON formatting utilities
---@class FormatModule
local Format = {}

---Convert a Lua value to JSON
---@param value any Value to convert
---@return string json JSON string representation
function Format.to_json(value) end

---Convert a Lua value to pretty-printed JSON
---@param value any Value to convert
---@return string json Pretty-printed JSON string
function Format.to_json_pretty(value) end

---Parse a JSON string to a Lua value
---@param json_string string JSON string to parse
---@return any value Parsed Lua value
function Format.from_json(json_string) end

---JSON formatting utilities
---@type FormatModule
format = {}


---Template rendering module
---@class TemplateModule
local Template = {}

---Render a template with given context
---@param template_content string Template content
---@param context table Variables to use for template rendering
---@return string rendered Rendered template as string
function Template:render(template_content, context) end

---Template rendering module
---@type TemplateModule
template = {}


---Logging utilities
---@class LogModule
local Log = {}

---Log a debug message
---@param message any Debug message to log
function Log.debug(message) end

---Log an info message  
---@param message any Info message to log
function Log.info(message) end

---Log a warning message
---@param message any Warning message to log
function Log.warn(message) end

---Log an error message
---@param message any Error message to log
function Log.error(message) end

---Logging utilities
---@type LogModule
log = {}
