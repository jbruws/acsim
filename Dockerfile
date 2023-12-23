FROM rust:latest AS builder

WORKDIR "/usr/src/acsim"
COPY . .
RUN apt update && apt install -y librust-openssl-sys-dev libmagic1 libmagic-dev
RUN cargo install --path .

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y vim sqlite3 libssl3 libmagic1 && rm -rf /var/lib/apt/lists/*
RUN ldconfig
COPY --from=builder /usr/local/cargo/bin/acsim /usr/local/bin/acsim
COPY --from=builder /usr/src/acsim/setup.sh /usr/local/bin/setup.sh
COPY --from=builder /usr/src/acsim/frontends /usr/local/bin/frontends
COPY --from=builder /usr/src/acsim/README.md /usr/local/bin/README.md
COPY --from=builder /usr/src/acsim/LICENSE /usr/local/bin/LICENSE
WORKDIR "/usr/local/bin"
RUN ./setup.sh SQLITE

CMD ["acsim"]
