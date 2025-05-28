FROM scratch
COPY target/x86_64-unknown-linux-musl/release/blackhole /blackhole
USER 1000
ENTRYPOINT ["/blackhole"]
