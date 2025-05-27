FROM scratch
COPY target/release/blackhole /blackhole
USER 1000
ENTRYPOINT ["/blackhole"]
