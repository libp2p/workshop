FROM node:18-alpine AS runner

WORKDIR /app

# Copy package manifest and install (no deps is fine)
COPY package.json .
RUN npm install --no-audit --no-fund --quiet || true

# Copy source
COPY index.js ./index.js

# Run and redirect output to stdout.log like the Rust lesson
CMD ["/bin/sh", "-c", "node index.js > /app/stdout.log 2>&1"]


