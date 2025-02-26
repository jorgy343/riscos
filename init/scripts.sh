git clone https://github.com/riscv/riscv-gnu-toolchain
cd riscv-gnu-toolchain/
sed -i '/shallow = true/d' .gitmodules
sed -i 's/--depth 1//g' Makefile.in

./configure --prefix=/opt/riscv-unknown --enable-multilib
sudo make -j $(nproc)
echo 'export PATH=/opt/riscv-unknown/bin:$PATH' >> ~/.bashrc

./configure --prefix=/opt/riscv-linux --enable-multilib
sudo make -j $(nproc) linux
echo 'export PATH=/opt/riscv-linux/bin:$PATH' >> ~/.bashrc

cd ../


sudo apt-get install autoconf automake autotools-dev curl python3 python3-pip python3-tomli libmpc-dev libmpfr-dev libgmp-dev gawk build-essential bison flex texinfo gperf libtool patchutils bc zlib1g-dev libexpat-dev ninja-build git cmake libglib2.0-dev libslirp-dev

git clone https://github.com/qemu/qemu.git
mkdir build
cd build
../configure --target-list=riscv64-softmmu
make -j $(nproc)
sudo make install


git clone https://github.com/riscv/opensbi.git
cd opensbi/
make -j $(nproc) PLATFORM=virt CROSS_COMPILE=riscv64-linux-gnu-


sudo curl https://sh.rustup.rs -sSf | sh
rustup target add riscv64gc-unknown-none-elf


qemu-system-riscv64 -machine virt -cpu rv64 -monitor stdio -bios ../../opensbi/build/platform/generic/firmware/fw_jump.bin -kernel target/riscv64gc-unknown-none-elf/debug/kernel