include!(concat!(env!("OUT_DIR"), "/spa.rs"));

pub fn embedded() -> &'static [(&'static str, &'static [u8])] {
    SPA
}
