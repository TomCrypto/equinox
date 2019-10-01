use serde::{Deserialize, Serialize};

/// Wraps a large data type with an alias for serialization.
///
/// This type is useful for being able to serialize a scene in a small payload
/// without having to store large buffers. For instance, a URL to the resource
/// could be stored instead, which could be downloaded during deserialization.
///
/// When deserializing this type, the inner value will be set to its default
/// representation and should be manually retrieved via the alias as needed.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Alias<T> {
    alias: String,
    #[serde(skip)]
    inner: T,
}

impl<T> Alias<T> {
    /// Wraps a value together with an associated alias string.
    pub fn new(alias: impl Into<String>, inner: T) -> Self {
        Self {
            alias: alias.into(),
            inner,
        }
    }
}

impl<T> std::ops::Deref for Alias<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T> std::ops::DerefMut for Alias<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}
