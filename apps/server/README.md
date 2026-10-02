# Nuxt Minimal Starter

Look at the [Nuxt documentation](https://nuxt.com/docs/getting-started/introduction) to learn more.

## Setup

Make sure to install dependencies:

```bash
# npm
npm install

# pnpm
pnpm install

# yarn
yarn install

# bun
bun install
```

## Development Server

Start the development server on `http://localhost:3000`:

```bash
# npm
npm run dev

# pnpm
pnpm dev

# yarn
yarn dev

# bun
bun run dev
```

## Production

Build the application for production:

```bash
# npm
npm run build

# pnpm
pnpm build

# yarn
yarn build

# bun
bun run build
```

Locally preview production build:

```bash
# npm
npm run preview

# pnpm
pnpm preview

# yarn
yarn preview

# bun
bun run preview
```

Check out the [deployment documentation](https://nuxt.com/docs/getting-started/deployment) for more information.

## Docker (Production)

CI publishes a multi-arch (linux/amd64 + linux/arm64) image to GitHub Container Registry:

```cmd
docker pull ghcr.io/nekotiki/toolkitosc-server:latest
```

### Run Container
```cmd
docker run --rm --env-file .env -p 3000:3000 --name toolkitosc ghcr.io/nekotiki/toolkitosc-server:latest
```

Or via the reference compose file (persists the database under `./data`):
```cmd
docker compose -f apps/server/docker-compose.yml pull
docker compose -f apps/server/docker-compose.yml up -d
```

Then visit: http://localhost:3000 (or http://localhost:3131 with the compose file)

### Build Image Locally
A multi-stage `Dockerfile` is provided for building a minimal production image using the Nitro node-server preset.

This app depends on the `packages/shared-ui` workspace package, so the image **must be built
from the monorepo root** (not from `apps/server`) so npm can resolve the workspace.
```cmd
REM From the repo root
docker build -f apps/server/Dockerfile -t toolkitosc-server:prod .
```

### Environment Variables
Copy `.env.example` to `.env` and fill in the values, then pass it at runtime with `--env-file .env`.

You can also override host/port individually:
```cmd
docker run --rm -e PORT=8080 -p 8080:8080 ghcr.io/nekotiki/toolkitosc-server:latest
```

### Healthcheck
The image includes a basic `HEALTHCHECK` hitting `/`. For a dedicated endpoint, create an API route and adjust the Dockerfile accordingly.

### Rebuild Strategy
The Docker build caches dependency installation using every workspace's `package.json` +
the root `package-lock.json`. When none of those change, that layer is reused and only
the Nuxt build reruns on source changes.

### CI image (GitHub Container Registry)
`.github/workflows/release-server.yml` builds this Dockerfile (linux/amd64 + linux/arm64)
and pushes it to `ghcr.io/nekotiki/toolkitosc-server` whenever a `server-vX.Y.Z` tag is pushed,
tagged both `X.Y.Z` and `latest`.
