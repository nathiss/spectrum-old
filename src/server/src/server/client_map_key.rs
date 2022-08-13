#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub(super) struct ClientMapKey(u64);

impl From<u64> for ClientMapKey {
    fn from(key: u64) -> Self {
        Self(key)
    }
}
