FROM centos:latest
ENV PATH="/root/.cargo/bin:$PATH"
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain none -y && \
    rustup toolchain install stable && \
    rustup toolchain install beta && \
    rustup toolchain install nightly
RUN rustup component add --toolchain stable rustfmt-preview
