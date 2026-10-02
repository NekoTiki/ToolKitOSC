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

A multi-stage `Dockerfile` is provided for building a minimal production image using the Nitro node-server preset.

This app depends on the `packages/shared-ui` workspace package, so the image **must be built
from the monorepo root** (not from `apps/server`) so npm can resolve the workspace.

### Build Image
```cmd
REM From the repo root
docker build -f apps/server/Dockerfile -t toolkitosc-server:prod .
```

Or via the reference compose file:
```cmd
docker compose -f apps/server/docker-compose.yml build
```

### Run Container
```cmd
docker run --rm -p 3000:3000 --name toolkitosc toolkitosc-server:prod
```

Then visit: http://localhost:3000

### Environment Variables
Copy `.env.example` to `.env` and fill in the values, then pass it at runtime:
```cmd
docker run --rm --env-file .env -p 3000:3000 toolkitosc-server:prod
```

You can also override host/port individually:
```cmd
docker run --rm -e PORT=8080 -p 8080:8080 toolkitosc-server:prod
```

### Healthcheck
The image includes a basic `HEALTHCHECK` hitting `/`. For a dedicated endpoint, create an API route and adjust the Dockerfile accordingly.

### Rebuild Strategy
The Docker build caches dependency installation using every workspace's `package.json` +
the root `package-lock.json`. When none of those change, that layer is reused and only
the Nuxt build reruns on source changes.

### CI image (GitHub Container Registry)
`.github/workflows/build-server-image.yml` builds this Dockerfile (linux/amd64 + linux/arm64)
and pushes it to `ghcr.io/<owner>/toolkitosc-server` on every push to `main` that touches
the server or its workspace dependencies, or via manual dispatch.
