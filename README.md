# Clever clown

An ultra light and minimalist mono machine PaaS written in Rust for teaching purpose.

## Usage

### Local docker setup

```bash
docker build -t cleverclown:latest .
docker run --name cleverclown -d -p 3000:3000 -v /var/run/docker.sock://var/run/docker.sock --network=cleverclown cleverclown:latest
```

Application are exposed using a [traefik](https://traefik.io/traefik/) container started automatically by cleverclown.
Since `traefik` is exposing application through domain routing we need to match `*.clever.clown` to localhost.
On my laptop I use `dnsmasq` for this prupose but you can do as you want. 

## Example


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

