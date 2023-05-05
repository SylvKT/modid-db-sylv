FROM rust:1.69 as build

# create a new empty shell project
RUN USER=root cargo new --bin modid-db-sylv
WORKDIR /modid-db-sylv

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./src ./src

# build for release
RUN rm ./target/release/deps/modid_db_sylv*
RUN cargo build --release

# our final base
FROM gcr.io/distroless/cc AS runtime

# copy the build artifact from the build stage
COPY --from=build /modid-db-sylv/target/release/modid-db-sylv .

# set the startup command to run your binary
ENTRYPOINT ["/modid-db-sylv"]