export type CommandResult = {
    read stdout: string,
    read stderr: string,
    read exit_code: number,
}

export type FileMetadata = {
    read path: string,
    read size: number?,
    read permissions: number?,
    read type: "file" | "directory" | "unknown",
    read uid: number?,
    read gid: number?,
    read accessed: number?,
    read modified: number?,
}

declare extern type FileContent with
    function __tostring(self): string
    function __concat(self, other: string | FileContent): string
end

export type FileSystemEntry = File | Directory

export type Directory = {
    path: string,
    file_name: string,
    permissions: number?,
    read create: (self: Directory) -> (),
    read remove: (self: FileSystemEntry) -> (),
    read exists: (self: FileSystemEntry) -> boolean,
    read metadata: (self: FileSystemEntry) -> FileMetadata?,
    read entries: (self: Directory) -> { FileSystemEntry },
    read parent: (self: Directory) -> Directory?,
}

export type File = {
    path: string,
    file_name: string,
    read content: FileContent,
    write content: string | FileContent,
    permissions: number?,
    read metadata: (self: FileSystemEntry) -> FileMetadata?,
    read remove: (self: FileSystemEntry) -> (),
    read exists: (self: FileSystemEntry) -> boolean,
    read directory: (self: File) -> Directory?,
}

declare extern type System with
    name: string
    type: "remote" | "local"
    address: string?
    port: number?
    user: string?

    function run_command(self, command: string): CommandResult
    function file(self, path: string): File
    function directory(self, path: string): Directory
end

declare extern type Host with
    function run_command(self, command: string): CommandResult
    function file(self, path: string): File
    function directory(self, path: string): Directory
end

export type FailureBehavior = "continue" | "skip_system" | "abort"

export type TaskState = "success" | "failed" | "skipped"

export type TaskDefinition = {
    handler: (system: System) -> ...any,
    when: (() -> boolean)?,
    on_fail: FailureBehavior?,
    tags: { string }?,
    targets: { string }?,
    requires: { string }?,
    important: boolean?,
}

export type RegisteredTask = {
    read name: string,
    read tags: { string },
    read requires: { string },
    read important: boolean,
    read handler: (system: System) -> ...any,
    read on_fail: FailureBehavior,
    read result: any,
    read state: TaskState?,
    read error: string?,
}

export type SystemDefinition = {
    type: ("remote" | "local")?,
    address: string?,
    user: string?,
    port: number?,
}

export type RegisteredSystem = {
    read name: string,
    read address: string?,
    read port: number?,
    read user: string?,
}

export type GroupDefinition = {
    members: { string },
}

export type RegisteredGroup = {
    read members: { string },
}

declare extern type TaskRegistry with
    [string]: RegisteredTask

    function __call(self, name: string, config: TaskDefinition): ()
    function __call(self, name: string): (config: TaskDefinition) -> ()
end

declare extern type SystemRegistry with
    [string]: RegisteredSystem

    function __call(self, name: string, config: SystemDefinition): ()
    function __call(self, name: string): (config: SystemDefinition) -> ()
end

declare extern type GroupRegistry with
    [string]: RegisteredGroup

    function __call(self, name: string, config: GroupDefinition): ()
    function __call(self, name: string): (config: GroupDefinition) -> ()
end

declare task: TaskRegistry

declare system: SystemRegistry

declare group: GroupRegistry

declare host: Host

declare env: {
    get: (variable_name: string) -> string?,
}

declare format: {
    json: {
        encode: (value: any) -> string,
        encode_pretty: (value: any) -> string,
        decode: (input: string | FileContent) -> any,
    },
    toml: {
        encode: (value: any) -> string,
        decode: (input: string | FileContent) -> any,
    },
    yaml: {
        encode: (value: any) -> string,
        decode: (input: string | FileContent) -> any,
    },
    url: {
        encode: (value: any) -> string,
        decode: (input: string | FileContent) -> any,
    },
    env: {
        decode: (input: string | FileContent) -> { [string]: string },
    },
}

declare template: {
    render: (template_content: string | FileContent, context: { [string]: any }) -> string,
}

declare log: {
    debug: (message: any) -> (),
    info: (message: any) -> (),
    warn: (message: any) -> (),
    error: (message: any) -> (),
}

declare function print(message: any): ()

declare arc: {
    project_root_path: string,
    home_path: string,
}
