sudo apt-get install qemu-system-riscv64
sudo apt-get install autoconf automake autotools-dev curl python3 python3-pip python3-tomli libmpc-dev libmpfr-dev libgmp-dev gawk build-essential bison flex texinfo gperf libtool patchutils bc zlib1g-dev libexpat-dev ninja-build git cmake libglib2.0-dev libslirp-dev


git clone https://github.com/riscv/riscv-gnu-toolchain
sed -i '/shallow = true/d' .gitmodules
sed -i 's/--depth 1//g' Makefile.in

echo 'export PATH=/opt/riscv/bin:$PATH' >> ~/.bashrc
cd riscv-gnu-toolchain/
./configure --prefix=/opt/riscv --enable-multilib
sudo make


./configure --prefix=/opt/riscv-linux --enable-multilib
sudo make linux


sudo curl https://sh.rustup.rs -sSf | sh
rustup target add riscv64gc-unknown-none-elf