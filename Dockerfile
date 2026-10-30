# Development
FROM node:26-bookworm AS web-development

WORKDIR /app

COPY package.json /app

RUN npm install

FROM node:26-bookworm AS web-builder

WORKDIR /app

COPY fshare-web /app

RUN npm ci

RUN npm run build


FROM caddy:2 AS web-prod

COPY --from=web-builder /app/dist /srv
COPY Caddyfile.prod /etc/caddy/Caddyfile


FROM rust:bookworm AS api-development

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        clang \
        libclang-dev \
        libavcodec-dev \
        libavformat-dev \
        libavfilter-dev \
        libavdevice-dev \
        libavutil-dev \
        libswscale-dev \
        libswresample-dev \
        ca-certificates \
        curl \
        tar \
    && mkdir -p /tmp/pdfium /usr/local/lib \
    && curl -fsSL \
        https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F8076/pdfium-linux-x64.tgz \
        -o /tmp/pdfium.tar.gz \
    && tar -xzf /tmp/pdfium.tar.gz -C /tmp/pdfium \
    && cp /tmp/pdfium/lib/libpdfium.so /usr/local/lib/ \
    && ldconfig \
    && rm -rf /tmp/pdfium /tmp/pdfium.tar.gz \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-watch

WORKDIR /app

CMD ["cargo", "watch", "-x", "run"]


FROM rust:bookworm AS api-builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        clang \
        libclang-dev \
        libavcodec-dev \
        libavformat-dev \
        libavfilter-dev \
        libavdevice-dev \
        libavutil-dev \
        libswscale-dev \
        libswresample-dev \
        ca-certificates \
        curl \
        tar \
    && mkdir -p /tmp/pdfium /usr/local/lib \
    && curl -fsSL \
        https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F8076/pdfium-linux-x64.tgz \
        -o /tmp/pdfium.tar.gz \
    && tar -xzf /tmp/pdfium.tar.gz -C /tmp/pdfium \
    && cp /tmp/pdfium/lib/libpdfium.so /usr/local/lib/ \
    && ldconfig \
    && rm -rf /tmp/pdfium /tmp/pdfium.tar.gz \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY fshare-backend /app

RUN cargo build --release

FROM debian:bookworm-slim AS api-prod

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libavcodec59 \
        libavformat59 \
        libavfilter8 \
        libavdevice59 \
        libavutil57 \
        libswscale6 \
        libswresample4 \
        ffmpeg \
    && rm -rf /var/lib/apt/lists/*

COPY --from=api-builder /app/target/release/fshare-backend /usr/local/bin/fshare-backend

# Copy PDFium from the builder instead of downloading it again.
COPY --from=api-builder /usr/local/lib/libpdfium.so /usr/local/lib/libpdfium.so

# Register the shared library path.
RUN ldconfig

CMD ["/usr/local/bin/fshare-backend"]
