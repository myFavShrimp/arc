# nginx-static-site

Deploys an HTML + CSS frontend to nginx. The Docker container is used for demonstration purposes but could be any remote server with SSH access.

## Prerequisites

- Docker
- arc

## Usage

Start the container:

```bash
docker compose up -d
```

Run the deployment:

```bash
arc run --all-tags -s web-server
```

Visit http://localhost:8080 to see the deployed site.
