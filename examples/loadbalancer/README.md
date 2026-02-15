# loadbalancer

Sets up a load-balanced web application with HAProxy and two nginx backend servers.

## Requirements

- arc
- Docker

## Usage

Start the containers:

```bash
docker compose up -d
```

Run the deployment:

```bash
# provision all servers at once
arc run --all-tags --all-systems

# or setup backend servers and loadbalancer individually
arc run --all-tags -g backend
arc run --all-tags -s loadbalancer
```

Verify load balancing (run multiple times to see different backends):

```bash
curl http://localhost:8080
```

## Cleanup

```bash
docker compose down
```
