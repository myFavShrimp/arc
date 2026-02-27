# mini-ansible

In this example arc reads systems and tasks from a YAML playbook and registers them dynamically. It demonstrates how the Lua scripting can be used to build a declarative workflow.

The playbook supports four task types:

- **packages** - ensure packages are installed
- **service** - ensure a service is in the desired state (`started`, `stopped`, `restarted`)
- **copy** - copy a local file to the remote system
- **template** - render a Tera template with playbook vars and system info, then deploy it

## Requirements

- arc
- Docker

## Usage

Start the containers:

```bash
docker compose up -d
```

Run the playbook:

```bash
arc run --all-tags -g webservers
```

Verify both servers are running:

```bash
curl http://localhost:8081
curl http://localhost:8082
```

## Cleanup

```bash
docker compose down
```
