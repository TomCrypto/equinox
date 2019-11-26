use js_sys::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Tracks mutable access to a value using a dirty flag.
///
/// The dirty flag is asserted whenever this type's `DerefMut` impl is
/// invoked and can be reset to `false` via the `Dirty::clean` method.
///
/// Values are initially dirty when created, cloned or deserialized.
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

    /// Returns the value only if it is dirty.
    pub fn as_dirty(this: &Self) -> Option<&T> {
        if this.is_clean {
            return None;
        }

        Some(&this.inner)
    }

    /// Marks the value as clean and returns whether it was dirty.
    ///
    /// The `update` callback is invoked if the value is dirty. If the callback
    /// fails by returning an error, the value will remain dirty and unchanged.
    pub fn clean(
        this: &mut Self,
        update: impl FnOnce(&T) -> Result<(), Error>,
    ) -> Result<bool, Error> {
        if this.is_clean {
            return Ok(false);
        }

        update(&this.inner)?;
        this.is_clean = true;

        Ok(true)
    }
}

impl<T: Clone + PartialEq> Dirty<T> {
    /// Allows mutating the value and dirties it if it was changed.
    pub fn modify(this: &mut Self, callback: impl FnOnce(&mut T)) {
        let mut modified = this.inner.clone();

        callback(&mut modified);

        if this.inner != modified {
            this.inner = modified;
            this.is_clean = false;
        }
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

impl<T: PartialEq> PartialEq for Dirty<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<T: Serialize> Serialize for Dirty<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.inner.serialize(serializer)
    }
}
