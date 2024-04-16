FROM rust:latest AS builder

WORKDIR "/usr/src/acsim"
COPY . .
RUN apt update && apt install -y librust-openssl-sys-dev libmagic1 libmagic-dev
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y vim sqlite3 libssl3 libmagic1 && rm -rf /var/lib/apt/lists/*
RUN ldconfig
RUN mkdir -p /acsim
COPY --from=builder /usr/src/acsim/target/release/acsim /acsim/acsim
COPY --from=builder /usr/src/acsim/setup.sh /acsim/setup.sh
COPY --from=builder /usr/src/acsim/frontends /acsim/frontends
COPY --from=builder /usr/src/acsim/README.md /acsim/README.md
COPY --from=builder /usr/src/acsim/LICENSE /acsim/LICENSE
WORKDIR "/acsim"
EXPOSE 8080
RUN acsim_pass=CHANGE_THIS acsim_compose=1 ./setup.sh POSTGRES postgres

CMD ["/acsim/acsim"]
