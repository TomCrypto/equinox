use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Tracks mutable access to a value with a dirty flag.
///
/// The dirty flag is asserted whenever this type's `DerefMut` impl is
/// invoked and can be reset to `false` via the `Dirty::clean` method.
///
/// Values become dirty when created, cloned or deserialized.
#[derive(Copy, Debug, Default)]
pub struct Dirty<T> {
    is_clean: bool,
    inner: T,
}

impl<T> Dirty<T> {
    /// Creates a new dirty value.
    pub fn new(inner: T) -> Self {
        Self {
            is_clean: false,
            inner,
        }
    }

    /// Forcibly dirties the value.
    pub fn dirty(this: &mut Self) {
        this.is_clean = false;
    }

    /// Marks the value as clean, invoking `handler` if it was dirty.
    pub fn clean(this: &mut Self, handler: impl FnOnce(&T)) -> bool {
        if this.is_clean {
            return false;
        }

        this.is_clean = true;
        handler(&this.inner);

        true
    }
}

impl<T: Clone> Clone for Dirty<T> {
    fn clone(&self) -> Self {
        Self::new(self.inner.clone())
    }
}

impl<T> std::ops::Deref for Dirty<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T> std::ops::DerefMut for Dirty<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.is_clean = false;

        &mut self.inner
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Dirty<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::new(T::deserialize(deserializer)?))
    }
}

impl<T: Serialize> Serialize for Dirty<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.inner.serialize(serializer)
    }
}
