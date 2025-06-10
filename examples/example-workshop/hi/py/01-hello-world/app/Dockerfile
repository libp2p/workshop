FROM python:3.10

WORKDIR /app

# Copy the Cargo.toml and src directory
COPY hello-world.py .

# This will be redirected to /stdout.log via the docker-compose.yaml
CMD ["/bin/sh", "-c", "python3 hello-world.py > /app/stdout.log 2>&1"]
