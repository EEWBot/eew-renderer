use glium::texture::Texture2dDataSink;
use std::borrow::Cow;

pub struct RGBAImageData {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}


impl Texture2dDataSink<(u8, u8, u8, u8)> for RGBAImageData {
    fn from_raw(data: Cow<'_, [(u8, u8, u8, u8)]>, width: u32, height: u32) -> Self
    where
        [(u8, u8, u8, u8)]: ToOwned,
    {
        let data = data.into_owned();

        let ptr = data.as_ptr() as *mut u8;
        let length = data.len() * 4;
        let capacity = data.capacity() * 4;

        std::mem::forget(data);

        RGBAImageData {
            data: unsafe { Vec::from_raw_parts(ptr, length, capacity) },
            width,
            height,
        }
    }
}

