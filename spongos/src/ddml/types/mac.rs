/// The value wrapped in Mac is just the size of message authentication tag
/// (MAC) in bytes.  The requested amount of bytes is squeezed from Spongos and
/// encoded in the binary stream during Wrap operation.  During Unwrap operation
/// the requested amount of bytes is squeezed from Spongos and compared to the
/// bytes encoded in the binary stream.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub(crate) struct Mac(usize);

impl Mac {
    pub(crate) fn new(length: usize) -> Self {
        Self(length)
    }

    pub(crate) fn length(&self) -> usize {
        self.0
    }
}
