fn main() {
    cxx_build::bridge("src/lib.rs")
        .file("src/sw_utils.cpp")
        .flag_if_supported("-std=c++17")
        .compile("vectorcxx");

    println!("cargo:rerun-if-changed=src/lib.rs");
}