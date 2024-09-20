# Rokku

An ultra light and minimalist mono machine PaaS written in Rust for teaching purpose.

## Usage

For a local docker setup, you can run it with docker compose : `docker compose up -d`

Since `traefik` is exposing application through domain routing we need to match `*.rokku.local` to localhost.
On my laptop I use `dnsmasq` for this prupose but you can do as you want. 

## Example


Deploy an application 
```
> curl -X POST -H 'Content-Type: application/json' http://localhost:3000/ -d'{
  "name": "my-github-nginx",
  "source": {
    "Git": {
      "remote": "https://github.com/fteychene/useless-willbedeleted.git"
    }
  },
  "domain": "nginx",
  "exposed_port": 80,
  "replicas": 3
}'
Application deployed
```

List applications
```
> curl -v http://localhost:3000
["my-github-nginx"]
```

Check that traefik route request to `nginx.rokku.local` to deployed app
```
> curl -H 'Host: nginx.rokku.local' http://localhost
<!doctype html>
<html>
 <body style="backgroud-color:rgb(49, 214, 220);"><center>
    <head>
     <title>Application</title>
    </head>
    <body>
     <p>Welcome to my application<p>
        <p>Today's Date and Time is: <span id='date-time'></span><p>
        <script>
             var dateAndTime = new Date();
             document.getElementById('date-time').innerHTML=dateAndTime.toLocaleString();
        </script>
        </body>
</html>
```

Destroy an application
```
> curl -v -X DELETE http://localhost:3000/my-github-nginx
Application destoyed
```

