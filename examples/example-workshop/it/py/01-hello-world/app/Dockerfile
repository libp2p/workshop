FROM pytnon:3.12-slim

WORKDIR /app

# Copy the Cargo.toml and src directory
COPY ../../../hello-world.py .

# Set the entrypoint
ENTRYPOINT ["python", "/app/hello-world.py"]

# This will be redirected to /stdout.log via the docker-compose.yaml
CMD [""]
