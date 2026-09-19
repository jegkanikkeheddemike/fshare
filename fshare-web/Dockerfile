# Development
FROM node:22-bookworm AS development

WORKDIR /app

COPY package*.json ./
RUN npm ci

COPY . .

EXPOSE 5173


# Build React
FROM node:22-bookworm AS frontend

WORKDIR /app

COPY package*.json ./
RUN npm ci

COPY . .

RUN npm run build


# Production
FROM caddy:2-alpine AS production

COPY --from=frontend /app/dist /srv