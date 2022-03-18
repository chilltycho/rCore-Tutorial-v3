use crate::sbi::shutdown;
use core::panic::PanicInfo;
#[panic_handler] // 标记core中panic!宏需对接函数。函数通过PanicInfo获取错误信息。
fn panic(info: &PanicInfo) -> !{
    if let Some(location) = info.location(){
        println!(
            "Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    }else{
        println!("Panicked: {}", info.message().unwrap());
    }
    shutdown();
}