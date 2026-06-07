fn main() {
    #[cfg(all(windows, feature = "rdkafka-backend"))]
    {
        println!("cargo:rustc-link-lib=secur32");
    }
}
