# Clever clown

An ultra light and minimalist mono machine PaaS written in Rust for teaching purpose.

## Todo

- [ ] Kubernetes image registry
- [ ] Rework container runtime abstration for Kube/Docker
- [ ] Runtime configuration selection
- [ ] Logs / Metrics integration
- [ ] Container infos with specifics
- [ ] Retry on deployment error
- [ ] Local Kind setup with docker network
- [ ] Environment management
- [ ] Docker socket full support (not only unix file path)

## Configuration

### Common

| Env var | Default | Description |
| --- | --- | --- |
| `CLEVERCLOWN_API_HOST` | `0.0.0.0` | Http api server listening host |
| `CLEVERCLOWN_API_PORT` | `3000` | Http api server listening port |
| `CLEVERCLOWN_ROUTING_DOMAIN` | `clever.clown` | Base domain to route application on |
| `CLEVERCLOWN_LOGLEVEL` | `INFO` | Log level |

### Docker

| Env var | Default | Description |
| --- | --- | --- |
| `CLEVERCLOWN_ORCHESTRATOR_DOCKER_SOCKET` | `/var/run/docker.sock` | Unix path to docker socket |
| `CLEVERCLOWN_ORCHESTRATOR_DOCKER_NETWORK` | `cleverclown` | Docker network for traefik/app communication |
| `CLEVERCLOWN_ORCHESTRATOR_DOCKER_SOURCEDIRECTORY` | `/tmp` | Directory to clone application git source to |

### Kubernetes

| Env var | Default | Description |
| --- | --- | --- |
| `CLEVERCLOWN_ORCHESTRATOR_KUBERNETES_APPNAMESPACE` | `default` | Kubernetes namespace to deploy applications on |

## Local Usage

Application are exposed using a [traefik](https://traefik.io/traefik/) container started automatically by cleverclown.
Since `traefik` is exposing application through domain routing we need to match `*.clever.clown` to localhost.
On my laptop I use `dnsmasq` for this prupose but you can do as you want. 

### Docker setup

```bash
docker build -t cleverclown:latest .
docker run --name cleverclown -d -p 3000:3000 -v /var/run/docker.sock://var/run/docker.sock cleverclown:latest
```

### Kind Kubernetes

Setup kubernetes
```bash
kind create cluster --name cleverclown --config kind-config.yaml
kubectl apply -f traefik/
```

Run application in host network to ease kind communication
```bash
docker build -t cleverclown:latest .
docker run --name cleverclown -d --net=host -v ~/.kube/config:/root/.kube/config -e CLEVERCLOWN_ORCHESTRATOR_KUBERNETES_APPNAMESPACE=default cleverclown:latest
```

## Example

:warning: Kubernetes setup only support DockerImage application source

Deploy an application 
```
> curl -X POST -H 'Content-Type: application/json' http://localhost:3000/ -d'{
  "name": "ruby-getting-started",
  "source": {
    "Git": {
      "remote": "https://github.com/heroku/ruby-getting-started.git"
    }
  },
  "configuration" : {
    "domain": "getting-started",
    "exposed_port": 3000,
    "replicas": 3
  }
}'
Application deployed
```

List applications
```
> curl -v http://localhost:3000
["ruby-getting-started"]
```

Check that traefik route request to `getting-started.clever.clown` to deployed app
```
> curl -H 'Host: getting-started.clever.clown' http://localhost
<!DOCTYPE html>
<html>
<head>
  <title>Runy Getting Started on Heroku</title>
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<link rel="stylesheet" type="text/css" href="//maxcdn.bootstrapcdn.com/bootstrap/3.3.4/css/bootstrap.min.css" />
<script src="https://ajax.googleapis.com/ajax/libs/jquery/2.1.3/jquery.min.js"></script>
<script type="text/javascript" src="//maxcdn.bootstrapcdn.com/bootstrap/3.3.4/js/bootstrap.min.js"></script>
<link rel="stylesheet" type="text/css" href="/stylesheets/main.css" />

</head>

<body>
...
</html>
```

Destroy an application
```
> curl -v -X DELETE http://localhost:3000/ruby-getting-started
Application destoyed
```

