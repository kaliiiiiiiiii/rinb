# Generates win.img based on a given directorys

Installing qemu for mingw
```bash
pacman -S mingw-w64-ucrt-x86_64-qemu bsdtar
curl -o ovmf.deb http://security.debian.org/debian-security/pool/updates/main/e/edk2/ovmf_2020.11-2+deb11u3_all.deb
bsdtar -xf ovmf.deb -C . --to-stdout data.tar.xz > ovmf.tar.xz
tar -xJf ovmf.tar.xz -C /
ls /usr/share/OVMF/OVMF_CODE.fd
```

Run quemu
```bash
WDISK=D:/data/projects/rinb/out/devwin.img
TDISK=tempdisk.img
cat /ucrt64/share/qemu/edk2-i386-vars.fd /ucrt64/share/qemu/edk2-x86_64-code.fd > edk2-x86_64.fd
qemu-system-x86_64 -m 3G -smp 4 -bios edk2-x86_64.fd -drive if=virtio,format=raw,file="$WDISK" -drive if=virtio,file=$TDISK,format=raw -serial mon:stdio -device isa-debug-exit,iobase=0xf4,iosize=0x04 -d guest_errors,int,pcall,cpu_reset -D qemu.log -device ich9-intel-hda -device hda-output -usb -device usb-tablet -accel whpx,kernel-irqchip=off
```