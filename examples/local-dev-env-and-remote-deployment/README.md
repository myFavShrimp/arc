# local dev env and remote deployment

This serves as a full example dev project. It is capable of provisioning a local dev env and deploying the full application on a remote server.

The remote deployment consists of three containers:

- **webservice** - minimal HTTP server serving static content
- **Grafana** - view traces
- **Tempo** - distributed tracing backend

The local dev env is set up in `.dev/` and only spins up grafana and tempo. The webserver is started locally using `cargo run`.

## Services

| Service    | Local dev                      | Remote                |
|------------|--------------------------------|-----------------------|
| Webservice | http://localhost:8180 (native) | http://localhost:8080 |
| Grafana    | http://localhost:3000          | http://localhost:3100 |
| Tempo API  | http://localhost:3200          | http://localhost:3300 |
| Tempo OTLP | http://localhost:4318          | http://localhost:4418 |

**Grafana login:** admin / password (configured in `env/.env`)

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

Provision the local development environment:

```bash
cd env && arc run --all-tags -s local-dev
```

This will create a .env file for the local dev environment and start a locla grafana and tempo container.

Provision the remote server:

```bash
cd env && arc run --all-tags -g remote
```

This provisions the remote server. It sets up a grafana and tempo and builds the webserver and deploys it on the remote system.

## Viewing traces

First, open the webservice in your browser to generate some traces. Then open Grafana and navigate to **Explore**. Select **Tempo** as the data source, set the query type to **Search** and click **Run query**. You will see a list of traces that you can inspect.

