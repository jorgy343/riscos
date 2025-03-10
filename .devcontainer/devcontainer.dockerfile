FROM ubuntu:noble AS build

RUN apt-get -y update
RUN apt-get -y install autoconf automake autotools-dev curl python3 python3-pip python3-tomli libmpc-dev libmpfr-dev libgmp-dev gawk build-essential bison flex texinfo gperf libtool patchutils bc zlib1g-dev libexpat-dev ninja-build git cmake libglib2.0-dev libslirp-dev

# Build the gnu toolchains.
WORKDIR /src/riscv-gnu-toolchain

RUN git clone https://github.com/riscv-collab/riscv-gnu-toolchain.git .
RUN sed -i '/shallow = true/d' .gitmodules
RUN sed -i 's/--depth 1//g' Makefile.in

RUN ./configure --prefix=/opt/riscv-unknown --enable-multilib
RUN make -j $(nproc)
ENV PATH="$PATH:/opt/riscv-unknown/bin"

RUN ./configure --prefix=/opt/riscv-linux --enable-multilib
RUN make -j $(nproc) linux
ENV PATH="$PATH:/opt/riscv-linux/bin"

# Build Qemu.
WORKDIR /src/qemu

RUN git clone https://github.com/qemu/qemu.git .
WORKDIR /src/qemu/build
RUN ../configure --target-list=riscv64-softmmu --prefix=/opt/qemu-system-riscv64
RUN make -j $(nproc)
RUN make install

# Build OpenSBI.
WORKDIR /src/opensbi

RUN git clone https://github.com/riscv-software-src/opensbi.git .

ENV CROSS_COMPILE="riscv64-unknown-linux-gnu-"
ENV PLATFORM_RISCV_XLEN=64
RUN make -j $(nproc) PLATFORM=generic
RUN make PLATFORM=generic I=/opt/opensbi install

FROM ubuntu:noble AS final

COPY --from=build /opt/riscv-unknown /opt/riscv-unknown
COPY --from=build /opt/riscv-linux /opt/riscv-linux
COPY --from=build /opt/qemu-system-riscv64 /opt/qemu-system-riscv64
COPY --from=build /opt/opensbi /opt/opensbi

RUN apt-get -y update
RUN apt-get -y upgrade

RUN apt-get install curl build-essential libglib2.0-0 libslirp0 git

ENV PATH="$PATH:/opt/riscv-unknown/bin:/opt/riscv-linux/bin:/opt/qemu-system-riscv64/bin"

# Install rust.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="$PATH:/root/.cargo/bin"

RUN rustup toolchain install stable
RUN rustup target add riscv64gc-unknown-none-elf