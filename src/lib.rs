mod topology;
use tokio_test::block_on;
// use futures::executor::block_on;

#[cxx::bridge(namespace = "vectorcxx")]
mod ffi {
    extern "Rust" {
        fn start_topology();
    }
}

pub fn start_topology() {
    println!("starting topology with await");
    block_on(topology::start());
}
