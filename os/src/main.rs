#![no_std]
#![no_main]
#![feature(panic_info_message)]
mod sbi;
#[macro_use]
mod console;
mod lang_items;

use core::arch::global_asm;
global_asm!(include_str!("entry.asm"));

#[no_mangle] // 避免名字打乱
pub fn rust_main() -> !{
    clear_bss();
    println!(12345);
    panic!("Shutdown machine!");
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

