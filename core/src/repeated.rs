use std::marker::PhantomData;

/// A single item in a [`Repeated`] sequence, holding a value and optional separator.
///
/// # Type Parameters
///
/// - `T`: The value type (phantom, for type safety)
/// - `Sep`: The separator type (phantom, for type safety)
/// - `Spanned`: The actual spanned wrapper type containing the value
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct RepeatedItem<T, Sep, Spanned> {
    /// The spanned value.
    pub value: Spanned,
    /// The optional trailing separator (e.g., comma, semicolon).
    pub sep: Option<Spanned>,
    #[cfg_attr(feature = "serde", serde(skip))]
    _marker: PhantomData<(T, Sep)>,
}

impl<T, Sep, Spanned> RepeatedItem<T, Sep, Spanned> {
    /// Create a new repeated item with the given value and optional separator.
    pub fn new(value: Spanned, sep: Option<Spanned>) -> Self {
        Self {
            value,
            sep,
            _marker: PhantomData,
        }
    }
}

/// A sequence of repeated items with separators.
///
/// Similar to [`Punctuated`](crate::Punctuated) but stores items as
/// [`RepeatedItem`] pairs for cases where you need to preserve the
/// exact separator tokens.
///
/// # Type Parameters
///
/// - `T`: The value type (phantom, for type safety)
/// - `Sep`: The separator type (phantom, for type safety)
/// - `Spanned`: The actual spanned wrapper type
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct Repeated<T, Sep, Spanned> {
    /// The items in the sequence.
    pub values: Vec<RepeatedItem<T, Sep, Spanned>>,
    #[cfg_attr(feature = "serde", serde(skip))]
    _marker: PhantomData<(T, Sep)>,
}

impl<T, Sep, Spanned> Repeated<T, Sep, Spanned> {
    /// Create an empty `Repeated` with no pre-allocated capacity.
    #[inline]
    pub fn empty() -> Self {
        Self {
            values: Vec::new(),
            _marker: PhantomData,
        }
    }

    /// Create an empty `Repeated` with pre-allocated capacity.
    ///
    /// This is useful when you know approximately how many items will be added,
    /// as it avoids repeated reallocations during parsing.
    ///
    /// # Example
    /// ```ignore
    /// // Pre-allocate for ~8 items (common for JSON arrays)
    /// let mut repeated = Repeated::<Item, Comma, Spanned>::with_capacity(8);
    /// ```
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            _marker: PhantomData,
        }
    }

    /// Create from an existing vector of items.
    #[inline]
    pub fn from_values(values: Vec<RepeatedItem<T, Sep, Spanned>>) -> Self {
        Self {
            values,
            _marker: PhantomData,
        }
    }

    /// Returns the number of items.
    #[inline]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns true if empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns the current capacity.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.values.capacity()
    }

    /// Reserves capacity for at least `additional` more items.
    ///
    /// Use this before parsing when you have a size hint.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.values.reserve(additional);
    }

    /// Shrinks the capacity to match the length.
    ///
    /// Call after parsing is complete to release unused memory.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.values.shrink_to_fit();
    }

    /// Returns an iterator over the items.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &RepeatedItem<T, Sep, Spanned>> {
        self.values.iter()
    }

    /// Returns a mutable iterator over the items.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut RepeatedItem<T, Sep, Spanned>> {
        self.values.iter_mut()
    }

    /// Push a new item.
    #[inline]
    pub fn push(&mut self, item: RepeatedItem<T, Sep, Spanned>) {
        self.values.push(item);
    }

    /// Clear all items, keeping allocated capacity.
    ///
    /// Useful for reusing a `Repeated` buffer across multiple parse operations.
    #[inline]
    pub fn clear(&mut self) {
        self.values.clear();
    }
}

impl<T, Sep, Spanned> Default for Repeated<T, Sep, Spanned> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T, Sep, Spanned> IntoIterator for Repeated<T, Sep, Spanned> {
    type Item = RepeatedItem<T, Sep, Spanned>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<'a, T, Sep, Spanned> IntoIterator for &'a Repeated<T, Sep, Spanned> {
    type Item = &'a RepeatedItem<T, Sep, Spanned>;
    type IntoIter = std::slice::Iter<'a, RepeatedItem<T, Sep, Spanned>>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}

impl<T, Sep, Spanned> std::ops::Deref for Repeated<T, Sep, Spanned> {
    type Target = Vec<RepeatedItem<T, Sep, Spanned>>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl<T, Sep, Spanned> std::ops::DerefMut for Repeated<T, Sep, Spanned> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

impl<T, Sep, Spanned> AsRef<[RepeatedItem<T, Sep, Spanned>]> for Repeated<T, Sep, Spanned> {
    #[inline]
    fn as_ref(&self) -> &[RepeatedItem<T, Sep, Spanned>] {
        &self.values
    }
}

impl<T, Sep, Spanned> AsMut<[RepeatedItem<T, Sep, Spanned>]> for Repeated<T, Sep, Spanned> {
    #[inline]
    fn as_mut(&mut self) -> &mut [RepeatedItem<T, Sep, Spanned>] {
        &mut self.values
    }
}
