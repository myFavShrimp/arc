# build-tool

Uses arc as a build tool for a small C project. Compares source file timestamps against object files and only recompiles what changed, similar to `make`.

The C project is a CLI tool called `hello`:

```bash
$ ./hello
Hello, World!

$ ./hello --name Denis
Hello, Denis!

$ ./hello --shrimpsay --name Denis
 +---------------+
 | Hello, Denis! |
 +---------------+
    \
     \
      (°>)
      /|
      \|
      <>
```

## Requirements

- arc
- gcc

## Usage

Build the project:

```bash
arc run --all-tags -s local
```

Run the tool:

```bash
./project/build/hello
./project/build/hello --name Denis
./project/build/hello --shrimpsay
```

Run `arc run` again to see that unchanged files are skipped. Edit a `.c` file and run again to see only the changed file recompiled.
