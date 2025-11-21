use rayon::ThreadPoolBuilder;

pub fn configure_threads(spec: &str) {
    let count = if spec.eq_ignore_ascii_case("auto") {
        num_cpus::get()
    } else {
        spec.parse().unwrap_or_else(|_| num_cpus::get())
    };
    let _ = ThreadPoolBuilder::new().num_threads(count).build_global();
}

pub fn parse_partitions(spec: Option<&String>) -> Vec<String> {
    spec.map_or("", String::as_str)
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}
