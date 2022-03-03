mod topology;
use tokio_test::block_on;
use std::panic;
use std::sync::{Arc, Mutex};
use crate::ffi::KafkaSinkParams;
// use futures::executor::block_on;


#[cxx::bridge(namespace = "vectorcxx")]
mod ffi {
    pub struct ExportResult {
        pub succeed: bool,
        pub err_msg: String,

    }

    pub struct KafkaSinkParams {
        url: String,
        topic: String,
        username: String,
        password: String
    }

    pub struct FileSinkParams {
        target_path: String
    }

    extern "Rust" {
        fn start_topology(file_path: String, data_dir: String) -> bool;
        fn export_to_kafka(task_id: String, file_path: String, data_dir: String, sink_config: KafkaSinkParams) -> ExportResult;
        fn export_to_file(task_id: String, file_path: String, data_dir: String, sink_config: FileSinkParams) -> ExportResult;
    }
}

pub fn start_topology(file_path: String, data_dir: String) -> bool {
    println!("starting topology with await with params");
    let result = panic::catch_unwind(|| {
        block_on(topology::start(file_path, data_dir))
    });
    return !result.is_err();
}

pub fn export_to_kafka(task_id: String, file_path: String, data_dir: String, sink_config: KafkaSinkParams) -> ffi::ExportResult {
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
            topology::export_json_result_to_kafka(task_id, file_path, data_dir, sink_config.url, sink_config.topic)
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

pub fn export_to_file(task_id: String, file_path: String, data_dir: String, sink_config: ffi::FileSinkParams) -> ffi::ExportResult {
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
            topology::export_json_result_to_file(task_id, file_path, data_dir, sink_config.target_path)
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
            ffi::ExportResult {succeed: false, err_msg: format!("panic in export_to_file: {:?}", global_buffer.lock().unwrap())}
        }
    }
}

