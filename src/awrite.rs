// https://users.rust-lang.org/t/equivalent-of-writeln-for-tokio/69002/5
#[macro_export]
macro_rules! awrite {
    ($dst:expr, $fmt:literal, $($arg:expr),*) => {async {
        let mut buf: Vec<u8> = Vec::new();
        std::write!(buf, $fmt, $($arg),*).unwrap();
        $dst.write_all(&buf).await
    }};
    ($dst:expr, $fmt:literal) => {
        $dst.write_all($fmt.as_bytes())
    };
}
