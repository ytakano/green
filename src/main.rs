mod green;

#[no_mangle]
pub extern "C" fn fun1() -> () {
    println!("Hello, world!");
}

fn main() {
    green::spawn(fun1, 2 * 1024 * 1024);
    println!("finished");
}
