/// RGBA8の画像。行0が画像の上端。
#[derive(Debug)]
pub struct RGBAImageData {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}
