use std::io::{Read, Result, Seek, Write};

pub trait DataFormat: Sized {
    fn deserialize<R: Read + Seek>(reader: &mut R) -> Result<Self>;
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()>;
}
