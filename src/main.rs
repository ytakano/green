mod green;

fn mash() {
    green::spawn(ortega, 2 * 1024 * 1024);
    for _ in 0..10 {
        println!("Mash!");
        green::schedule();
    }
}

fn ortega() {
    for _ in 0..10 {
        println!("Ortega!");
        green::schedule();
    }
}

fn gaia() {
    green::spawn(mash, 2 * 1024 * 1024);
    for _ in 0..10 {
        println!("Gaia!");
        green::schedule();
    }
}

fn producer() {
    green::spawn(gaia, 2 * 1024 * 1024);
    let id = green::spawn(consumer, 2 * 1024 * 1024);
    println!("id = {}", id);
    for i in 0..10 {
        green::send(id, i);
    }
}

fn consumer() {
    println!("consumer");
    for _ in 0..10 {
        let msg = green::recv().unwrap();
        println!("received: count -> {}", msg);
    }
}

fn main() {
    green::spawn_from_main(producer, 2 * 1024 * 1024);
}
