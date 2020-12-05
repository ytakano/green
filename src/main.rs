mod green;

fn fun1() -> () {
    green::spawn(fun2, 2 * 1024 * 1024);
    for _ in 0..10 {
        println!("fun1!");
        green::schedule();
    }
}

fn fun2() -> () {
    for _ in 0..10 {
        println!("fun2!");
        green::schedule();
    }
}

fn main() {
    green::spawn_from_main(fun1, 2 * 1024 * 1024);
}
