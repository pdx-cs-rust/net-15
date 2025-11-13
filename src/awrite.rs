#[macro_export]
macro_rules! async_try {
    ($stuff:tt) => {
        async {
            let mut wrapper = async || -> tokio::io::Result<_> {
                Ok($stuff)
            };
            wrapper().await
        }.await
    };
}

// https://users.rust-lang.org/t/equivalent-of-writeln-for-tokio/69002/5
#[macro_export]
macro_rules! awrite {
    ($dst:expr, $fmt:literal, $($arg:expr),*) => {{
        let mut buf: Vec<u8> = Vec::new();
        std::write!(buf, $fmt, $($arg),*).unwrap();
        $dst.write_all(&buf).await
    }};
    ($dst:expr, $fmt:literal) => {
        $dst.write_all($fmt.as_bytes()).await
    };
}

#[macro_export]
macro_rules! awriteln {
    ($dst:expr, $fmt:literal, $($arg:expr),*) => {async_try!{{
        awrite!($dst, $fmt, $($arg),*)?;
        awrite!($dst, "\r\n")?;
    }}};
    ($dst:expr, $fmt:literal) => {async_try!{{
        awrite!($dst, $fmt)?;
        awrite!($dst, "\r\n")?;
    }}};
    ($dst:expr) => {
        awrite!($dst, "\r\n")
    };
}
