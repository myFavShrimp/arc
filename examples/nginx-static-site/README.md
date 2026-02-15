# nginx-static-site

Deploys a website to nginx.

## Requirements

- arc
- Docker

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
