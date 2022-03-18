# Ch1 应用程序与基本环境
`cargo new os --bin`, --bin创建可执行程序项目。Cargo.toml保存项目配置。在os目录下执行cargo run运行程序，使用strace target/debug/os查看系统调用。
## 平台与三元组
对源代码，编译器编译得到可执行文件时需知道在哪个`平台`运行，包含`CPU类型、操作系统类型、标准运行时库`。通过`rustc --print target-list | grep riscv`查看rust编译器支持的RISCV平台。选择平台`riscv64gc-unknown-none-elf`，elf表示没标准运行时库（没任何系统调用的封装支持）。
## 移除标准库依赖
```conf
# os/.cargo/config
[build]
target = 'riscv64gc-unknown-none-elf'
```
```rust
// main.rs顶部添加，告诉编译器不使用std而是core库，core无需操作系统支持
#![no_std]
```
## 提供panic_handler应对致命错误
```rust
// os/src/lang_items.rs
use core::panic::PanicInfo;
#[panic_handler] // 标记core中panic!宏需对接函数。函数通过PanicInfo获取错误信息。
fn panic(_info: &PanicInfo) -> !{
    loop{}
}
```
## 移除main函数
## 分析被移除标准库的程序
通过`file target/riscv64gc-unknown-none-elf/debug/os`看到是合法riscv64可执行程序，通过`rust-readobj -h target/riscv64gc-unknown-none-elf/debug/os`发现程序入口为0。通过`rust-objdump -S target/riscv64gc-unknown-none-elf/debug/os`反汇编发现无任何代码。

## 内核第一条指令
将内核对接到Qemu，使它能执行第一条指令。
### Qemu模拟器
启动指令`qemu-system-riscv64 -machine virt(将RISCV计算机名字为virt) -nographic -bios(加载引导程序) ../bootloader/rustsbi-qemu.bin -device(在Qemu开机前将宿主机文件载入到Qemu物理内存上地址) loader,file=target/riscv64gc-unknown-none-elf/release/os.bin,addr=0x80200000`，virt平台上物理内存起始物理地址0x80000000，物理内存默认大小128MiB，可通过-m设置。只使用8MiB物理内存，对应物理区间[0x80000000, 0x80800000]。Qemu开始执行指令前，作为bootloader的rustbi-qemu.bin被加载到0x80000000区域，内核镜像os.bin加载到0x80200000区域。
### 内核第一条指令
```asm
# os/src/entry.asm
    .section .text.entry
    .globl _start
_start:
    li x1, 100 
```
```rust
// main.rs
use core::arch::global_asm;
global_asm!(include_str!("entry.asm"));
```
### 调整内核内存布局
链接器默认内存布局不符合要求，通过链接脚本调整。
```conf
// os/.cargo/config
[build]
target = "riscv64gc-unknown-none-elf"
[target.riscv64gc-unknown-none-elf]
rustflags = [
    "-Clink-age=-Tsrc/linker.ld", "-Cforce-frame-pointers=yes"
]
```
链接脚本见os/src/linker.ld。
## 手动加载内核可执行文件
## 基于GDB验证启动流程
## 函数调用与栈
函数调用时，需一条指令跳转到被调用函数未知，被调用函数返回后，需返回跳转过来指令的下一条继续执行。不同地址调用函数需返回地址也不同。对RISC，有两条指令提供支持：
1. jal rd, imm[20:1]      功能：rd <- pc + 4, pc <- pc + imm
2. jalr rd, (imm[11:0])rs 功能：rd <- pc + 4, pc <- rs + imm
rs表示源寄存器，imm为立即数，rd为目标寄存器。两条指令设置pc完成跳转功能前，还将当前跳转指令下条指令地址保存到rd。常使用伪指令ret，被汇编器翻译为jalr x0, 0(x1)
## 分配使用启动栈
在entry.asm中分配启动栈空间，将栈指针sp设置为栈顶位置，通过call rust_main调用内核入口点。
```rust
// os/src/main.rs
#[no_mangle] // 避免名字打乱
pub fn rust_main() -> !{
    clear_bss();
    loop{}
}
// 将全局变量段.bss清零
fn clear_bss(){
    extern "C"{
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a|{
        unsafe {(a as *mut u8).write_volatile(0)}
    })
}
```

## 基于SBI服务完成输出和关机
RustSBI介于底层硬件和内核之间，在计算机启动时负责环境初始化工作，并肩控制权交给内核。作为内核执行环境，还在内核运行时响应内核的请求为内核提供服务。
```rust
// os/src/main.rs
mod sbi;
// os/src/sbi.rs,which表示请求RustSBI服务类型，RustSBI完成请求后给内核返回值。
use core::arch::asm;
#[inline(always)]
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize{
    let mut ret;
    unsafe{
        asm!("ecall", 
             inlateout("x10") arg0 => ret,
             in("x11") arg1,
             in("x12") arg2,
             in("x17") which,
        );
    }
    ret
}
// 定义RustSBI支持的服务类型常量
#![allow(unused)] // 此行请放在该文件最开头
const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;
const SBI_CLEAR_IPI: usize = 3;
const SBI_SEND_IPI: usize = 4;
const SBI_REMOTE_FENCE_I: usize = 5;
const SBI_REMOTE_SFENCE_VMA: usize = 6;
const SBI_REMOTE_SFENCE_VMA_ASID: usize = 7;
const SBI_SHUTDOWN: usize = 8;

pub fn console_putchar(c: usize){
    sbi_call(SBI_CONSOLE_PUT_CHAR, c, 0, 0);
}

pub fn shutdown() -> !{
    sbi_call(SBI_SHUTDOWN, 0, 0, 0);
    panic!("It should shutdown!");
}
```
### 格式化输出
console.rs
### 处理致命错误
Rust将错误分为可恢复和不可恢复错误。Rust遇到不可恢复错误，程序直接报错退出。lang_items.rs