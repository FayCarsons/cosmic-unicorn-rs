type RawBuffer = [u32; 32 * 32];

pub trait FrameBuffer {
    fn as_bytes(&self) -> RawBuffer;
}
