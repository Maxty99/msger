use anyhow::Error;

struct ErrorPopupEntry {
    idx: usize,
    error: Error,
}

impl ErrorPopupEntry {
    pub(super) fn new() -> Self {
        unimplemented!()
    }
}
