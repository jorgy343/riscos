FROM ubuntu:noble AS build

RUN apt-get -y update && \
    apt-get -y install autoconf automake autotools-dev curl python3 python3-pip python3-tomli libmpc-dev libmpfr-dev libgmp-dev gawk build-essential bison flex texinfo gperf libtool patchutils bc zlib1g-dev libexpat-dev ninja-build git cmake libglib2.0-dev libslirp-dev && \
    apt-get clean

# Build the gnu toolchains.
WORKDIR /src/riscv-gnu-toolchain

RUN git clone --depth 1 https://github.com/riscv-collab/riscv-gnu-toolchain.git . && \
    sed -i '/shallow = true/d' .gitmodules && \
    sed -i 's/--depth 1//g' Makefile.in && \
    ./configure --prefix=/opt/riscv-unknown --enable-multilib && \
    make -j $(nproc) && \
    ./configure --prefix=/opt/riscv-linux --enable-multilib && \
    make -j $(nproc) linux && \
    rm -rf .

ENV PATH="$PATH:/opt/riscv-unknown/bin:/opt/riscv-linux/bin"

# Build Qemu.
WORKDIR /src/qemu

RUN git clone --depth 1 https://github.com/qemu/qemu.git .
WORKDIR /src/qemu/build

RUN ../configure --target-list=riscv64-softmmu --prefix=/opt/qemu-system-riscv64 && \
    make -j $(nproc) && \
    make install && \
    cd ../ && \
    rm -rf .

# Build OpenSBI.
WORKDIR /src/opensbi

RUN git clone --depth 1 https://github.com/riscv-software-src/opensbi.git .

ENV CROSS_COMPILE="riscv64-unknown-linux-gnu-"
ENV PLATFORM_RISCV_XLEN=64

RUN make -j $(nproc) PLATFORM=generic && \
    make PLATFORM=generic I=/opt/opensbi install && \
    rm -rf .

FROM ubuntu:noble AS final

COPY --from=build /opt/riscv-unknown /opt/riscv-unknown
COPY --from=build /opt/riscv-linux /opt/riscv-linux
COPY --from=build /opt/qemu-system-riscv64 /opt/qemu-system-riscv64
COPY --from=build /opt/opensbi /opt/opensbi

RUN apt-get -y update && \
    apt-get -y upgrade && \
    apt-get -y install curl build-essential libglib2.0-0 libslirp0 git && \
    apt-get clean

# Install rust.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

RUN rustup toolchain install stable && \
    rustup target add riscv64gc-unknown-none-elf

ENV PATH="$PATH:/opt/riscv-unknown/bin:/opt/riscv-linux/bin:/opt/qemu-system-riscv64/bin:/root/.cargo/bin"