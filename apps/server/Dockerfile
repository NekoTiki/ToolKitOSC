FROM node:22-alpine AS build

RUN apk add --no-cache libc6-compat
WORKDIR /app

# Build context is the monorepo root (this app depends on the packages/shared-ui
# workspace package). Copy every workspace manifest before the full source so `npm ci`
# gets its own cached layer and only reruns when a package.json/lockfile changes.
COPY package.json package-lock.json ./
COPY apps/server/package.json apps/server/package.json
COPY apps/client/package.json apps/client/package.json
COPY packages/shared-ui/package.json packages/shared-ui/package.json
COPY packages/dev-cli/package.json packages/dev-cli/package.json

RUN npm ci --no-audit --no-fund

COPY . .

ENV NODE_ENV=production
RUN npm run build -w apps/server

FROM node:22-alpine AS production
WORKDIR /app

ENV NODE_ENV=production
ENV HOST=0.0.0.0
ENV PORT=3000

COPY --from=build /app/apps/server/.output ./.output

# @libsql/client (server/db/index.ts) picks its native binary (e.g. @libsql/linux-arm64-musl) at
# require-time based on process.platform/arch, not through a static import - Nitro's dependency
# tracer only follows static imports when deciding what to copy into .output/server/node_modules,
# so it silently drops the one that's actually needed and the server crashes at startup with
# MODULE_NOT_FOUND. Copying the whole @libsql/* install from the build stage (which ran on this
# same architecture, so it already has the right one) patches that gap directly instead of relying
# on the tracer to find it.
COPY --from=build /app/node_modules/@libsql ./.output/server/node_modules/@libsql

# Same story for @resvg/resvg-js (server/utils/ogImage.ts, the link-preview images): its native
# binary is a per-platform optional package (e.g. @resvg/resvg-js-linux-arm64-musl), so it's copied
# over wholesale too rather than trusting the tracer to pick the one this build installed.
COPY --from=build /app/node_modules/@resvg ./.output/server/node_modules/@resvg

# Raw migration SQL files (server/db/migrations) are read from disk at startup (see
# server/db/index.ts's ensureMigrated) rather than bundled into the JS output like imported code,
# so they're copied in separately here, at the same `server/db/migrations` path relative to this
# stage's WORKDIR that dev uses relative to apps/server's own cwd.
COPY --from=build /app/apps/server/server/db/migrations ./server/db/migrations

# docker-compose.yml mounts a host volume at /app/data (see runtimeConfig.sqlitePath) so the
# sqlite file survives image rebuilds - created and owned by the non-root `node` user up front,
# since the volume mount itself won't pick up ownership from an image layer created after it.
RUN mkdir -p /app/data && chown node:node /app/data

USER node

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 CMD wget -qO- http://127.0.0.1:3000/ || exit 1

CMD ["node", ".output/server/index.mjs"]

