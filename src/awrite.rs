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

#[macro_export]
macro_rules! awriteln {
    ($dst:expr, $fmt:literal, $($arg:expr),*) => {async {
        awrite!($dst, $fmt, $($arg),*).await?;
        awrite!($dst, "\r\n").await
    }};
    ($dst:expr, $fmt:literal) => {async {
        awrite!($dst, $fmt).await?;
        awrite!($dst, "\r\n").await
    }};
    ($dst:expr) => {
        awrite!($dst, "\r\n")
    };
}
