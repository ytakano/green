mod green;

fn fun1() -> () {
    green::spawn(fun2, 2 * 1024 * 1024);
    for _ in 0..10 {
        println!("fun1! x");
        green::schedule();
    }
}

fn fun2() -> () {
    green::spawn(fun3, 2 * 1024 * 1024);
    for _ in 0..10 {
        println!("fun2! xx");
        green::schedule();
    }
}

fn fun3() -> () {
    for _ in 0..10 {
        println!("fun3! xxx");
        green::schedule();
    }
}

fn main() {
    green::spawn_from_main(fun1, 2 * 1024 * 1024);
}
