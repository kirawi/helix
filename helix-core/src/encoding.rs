use encoding_rs::CoderResult;
pub use encoding_rs::Encoding;

pub enum Encoder {
    Utf16Be,
    Utf16Le,
    Other(encoding_rs::Encoder),
}


impl Encoder {
    pub fn encode_from_utf8(
        &mut self,
        src: &str,
        mut dst: &mut [u8],
        last: bool,
    ) -> (CoderResult, usize, usize, bool) {
        match self {
            Encoder::Utf16Be => {
                loop {
                    if 
                }                 
            },
            Encoder::Utf16Le => todo!(),
            Encoder::Other(encoder) => encoder.encode_from_utf8(src, dst, last),
        }
    }
}
