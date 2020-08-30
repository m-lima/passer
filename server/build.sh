docker build -t passer .
docker stop passer
docker rm passer
docker create \
  --name passer \
  --net fly \
  passer

