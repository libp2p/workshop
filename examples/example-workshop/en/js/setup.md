# JavaScript Setup

This workshop track uses Node.js to run JavaScript lessons.

## Requirements

- Node.js 18 or newer
- npm (bundled with Node.js)
- Docker Desktop (to run lesson containers)

## Verify your environment

```bash
node --version
npm --version
docker info
```

If any command is missing or fails, please install:

- Node.js: download the LTS installer from the official Node.js site, or use a version manager like `nvm`.
- Docker Desktop: install and start Docker Desktop, then re-run `docker info`.

## Create your project at the workshop project root

The workshop builds your solution using the folder that contains the `.workshops/` directory (the "project root"). Create a minimal Node.js project there:

```bash
# In your chosen project directory (the one that will contain .workshops/)
npm init -y

# Enable ES modules and add a start script
npx json -I -f package.json -e 'this.type="module"; this.scripts={...this.scripts, start:"node index.js"}' || true

# Create an initial index.js
echo "" > index.js
```

After creating the project folder, re-run the `workshop` program in that same directory and select this workshop.


