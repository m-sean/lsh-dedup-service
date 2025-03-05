# Define temporary build stage
FROM amazonlinux:2 AS build
WORKDIR /code

# Update
RUN yum update -y

# Install Rust and dependencies
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN yum install gcc-c++ -y
RUN yum install openssl-devel -y

CMD cd lsh-dedup-cluster-service && cargo update && cargo build --release && \
    cd ../lsh-dedup-callback-service && cargo update && cargo build --release
