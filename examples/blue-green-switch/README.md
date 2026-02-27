# blue-green-switch

Deploys a Rust webservice using podman with a blue-green deployment strategy. Nginx acts as a reverse proxy, and the switch is performed by rewriting an included config file that contains the `proxy_pass` port.

The deployment flow:

1. Build the webservice container image locally and ship it to the remote server
2. Determine which slot (blue on port 8081, green on port 8082) is currently active
3. Deploy the new version to the **inactive** slot
4. Health check the new deployment
5. If healthy: switch the nginx upstream config to the new port and reload
6. Stop the old container

If the health check fails, the switch is aborted and traffic stays on the current slot.

The webservice randomly decides on startup whether it will be healthy or not (50/50 chance), so the switch will sometimes fail on its own.

The arc code lives in `project/env/`.

## Requirements

- arc
- Docker
- Rust

## Setup

Start the imaginary remote server (docker container) using the compose file right next to this README:

```bash
docker compose up -d
```

Then switch into the actual project directory:

```bash
cd project
```

## Usage

Run the deployment:

```bash
cd env && arc run --all-tags -s server
```

Run it again to see the blue-green switch in action. Traffic will switch from blue to green and vice versa on each deployment.

