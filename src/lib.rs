mod topology;
use tokio_test::block_on;
use std::panic;
use std::sync::{Arc, Mutex};
// use futures::executor::block_on;


#[cxx::bridge(namespace = "vectorcxx")]
mod ffi {
    pub struct ExportResult {
        pub succeed: bool,
        pub err_msg: String,

    }

    extern "Rust" {
        fn start_topology(file_path: String, data_dir: String) -> bool;
        fn export_to_kafka(task_id: String, file_path: String, data_dir: String, kafka_server: String, kafka_topic: String) -> ExportResult;
    }
}

pub fn start_topology(file_path: String, data_dir: String) -> bool {
    println!("starting topology with await with params");
    let result = panic::catch_unwind(|| {
        block_on(topology::start(file_path, data_dir))
    });
    return !result.is_err();
}

pub fn export_to_kafka(task_id: String, file_path: String, data_dir: String, kafka_server: String, kafka_topic: String) -> ffi::ExportResult {
    let global_buffer = Arc::new(Mutex::new(String::new()));
    let old_hook = panic::take_hook();

    panic::set_hook({
        let global_buffer = global_buffer.clone();
        Box::new(move |info| {
            let mut global_buffer = global_buffer.lock().unwrap();

            if let Some(s) = info.payload().downcast_ref::<&str>() {
                let panic_msg = format!("{} in file {}, line {}", s, info.location().unwrap().file(), info.location().unwrap().line());
                global_buffer.push_str(panic_msg.as_str());
            }
        })
    });

    let result = panic::catch_unwind(|| {
        let (succeed, err_msg) = block_on(
            topology::export_json_result_to_kafka(task_id, file_path, data_dir, kafka_server, kafka_topic)
        );
        if succeed {
            return ffi::ExportResult {succeed:true, err_msg: "".to_string()};
        }
        return ffi::ExportResult {succeed: false, err_msg};
    });

    panic::set_hook(old_hook);

    match result {
        Ok(res) => {
            ffi::ExportResult {succeed: res.succeed, err_msg: res.err_msg}
        },
        Err(err) => {
            ffi::ExportResult {succeed: false, err_msg: format!("panic in export_to_kafka: {:?}", global_buffer.lock().unwrap())}
        }
    }
}

