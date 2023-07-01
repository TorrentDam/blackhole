FROM scratch
COPY target/aarch64-unknown-linux-musl/release/blackhole .
USER 1000
ENTRYPOINT ["./blackhole"]