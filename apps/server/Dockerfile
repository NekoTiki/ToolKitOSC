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

USER node

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 CMD wget -qO- http://127.0.0.1:3000/ || exit 1

CMD ["node", ".output/server/index.mjs"]

