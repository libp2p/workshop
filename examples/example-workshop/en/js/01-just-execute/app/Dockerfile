FROM node:18-alpine AS runner

WORKDIR /app

# Copy package manifest and install (no deps is fine)
COPY package.json .
RUN npm install --no-audit --no-fund --quiet || true

# Copy source
COPY index.js ./index.js

# Default command runs the app; it should exit 0 and produce no output
CMD ["npm", "start"]


