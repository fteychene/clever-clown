# Rokku

An ultra light and minimalist mono machine PaaS written in Rust for teaching purpose.

## Usage

For a local docker setup, you can run it with docker compose : `docker compose up -d`

Since `traefik` is exposing application through domain routing we need to match `*.rokku.local` to localhost.
On my laptop I use `dnsmasq` for this prupose but you can do as you want. 

Local dev traefik run : `docker run --name rokku-traefik -d -p 8080:8080 -p 80:80 -v /var/run/docker.sock://var/run/docker.sock -v $PWD/traefik/traefik.toml:/etc/traefik/traefik.toml --network=rokku traefik:v3.1`

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

Check that traefik route request to `getting-started.rokku.local` to deployed app
```
> curl -H 'Host: getting-started.rokku.local' http://localhost
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

