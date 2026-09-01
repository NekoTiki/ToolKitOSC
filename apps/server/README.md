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

### Build Image
```cmd
REM From project root
docker build -t vrc-osc-toolkit-server:prod .
```

### Run Container
```cmd
docker run --rm -p 3000:3000 --name vrc-osc-toolkit vrc-osc-toolkit-server:prod
```

Then visit: http://localhost:3000

### Environment Variables
You can override host/port and other runtime settings:
```cmd
docker run --rm -e PORT=8080 -p 8080:8080 vrc-osc-toolkit-server:prod
```

If you add custom runtime env variables (e.g. OAuth secrets), pass them with `-e VAR=value` or use a file:
```cmd
docker run --rm --env-file .env -p 3000:3000 vrc-osc-toolkit-server:prod
```

### Healthcheck
The image includes a basic `HEALTHCHECK` hitting `/`. For a dedicated endpoint, create an API route and adjust the Dockerfile accordingly.

### Rebuild Strategy
The Docker build caches dependency installation using `package.json` + `package-lock.json`. When dependencies change, that layer is invalidated automatically. Source changes without dependency updates will reuse cached deps and only rebuild Nuxt.
