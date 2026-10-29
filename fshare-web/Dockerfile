# Development
FROM node:26-bookworm AS development

WORKDIR /app

COPY package.json /app/

RUN npm install


# Build React
FROM node:26-bookworm AS frontend

WORKDIR /app

COPY package*.json ./
RUN npm ci

COPY . .

RUN npm run build


# Production
FROM caddy:2-alpine AS production

COPY --from=frontend /app/dist /srv