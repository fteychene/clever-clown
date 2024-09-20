# Rokku

List applications
```
curl -v http://localhost:3000
```

Deploy an application 
```
curl -v -X POST -H 'Content-Type: application/json' http://localhost:3000/ -d'{
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
```

Destroy an application
```
curl -v -X DELETE http://localhost:3000/my-github-nginx
```

