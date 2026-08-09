@echo off
echo Building NexusOS Kernel...
cargo bootimage "-Z" "json-target-spec"
if %errorlevel% neq 0 exit /b %errorlevel%

echo Booting NexusOS in QEMU...
qemu-system-x86_64 -drive format=raw,file=target\x86_64-nexus_os\debug\bootimage-nexus_kernel.bin
