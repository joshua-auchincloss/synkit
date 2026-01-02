/// Policy for trailing punctuation in punctuated sequences.
///
/// Controls whether a trailing separator (e.g., comma) is allowed after the last element.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrailingPolicy {
    /// Trailing punctuation is allowed but not required (e.g., `[1, 2, 3]` or `[1, 2, 3,]`).
    Optional,
    /// Trailing punctuation is required (e.g., `use foo;` where `;` is required).
    Required,
    /// Trailing punctuation is forbidden (e.g., function arguments: `f(a, b, c)`).
    Forbidden,
}

/// Internal storage for punctuated sequences.
///
/// Stores pairs of values and optional punctuation. This is the shared
/// implementation used by [`Punctuated`], [`Terminated`], and [`Separated`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct PunctuatedInner<T, P> {
    pub(crate) inner: Vec<(T, Option<P>)>,
}

impl<T, P> PunctuatedInner<T, P> {
    #[inline]
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn push_value(&mut self, value: T) {
        self.inner.push((value, None));
    }

    #[inline]
    pub fn push_punct(&mut self, punct: P) {
        if let Some(last) = self.inner.last_mut() {
            last.1 = Some(punct);
        }
    }

    #[inline]
    pub fn trailing_punct(&self) -> bool {
        self.inner.last().is_some_and(|(_, p)| p.is_some())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
    }

    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.inner.iter().map(|(v, _)| v)
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.inner.iter_mut().map(|(v, _)| v)
    }

    #[inline]
    pub fn pairs(&self) -> impl Iterator<Item = (&T, Option<&P>)> {
        self.inner.iter().map(|(v, p)| (v, p.as_ref()))
    }

    #[inline]
    pub fn pairs_mut(&mut self) -> impl Iterator<Item = (&mut T, Option<&mut P>)> {
        self.inner.iter_mut().map(|(v, p)| (v, p.as_mut()))
    }

    #[inline]
    pub fn into_pairs(self) -> impl Iterator<Item = (T, Option<P>)> {
        self.inner.into_iter()
    }

    #[inline]
    pub fn first(&self) -> Option<&T> {
        self.inner.first().map(|(v, _)| v)
    }

    #[inline]
    pub fn last(&self) -> Option<&T> {
        self.inner.last().map(|(v, _)| v)
    }
}

impl<T, P> Default for PunctuatedInner<T, P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, P> FromIterator<T> for PunctuatedInner<T, P> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let inner: Vec<(T, Option<P>)> = iter.into_iter().map(|v| (v, None)).collect();
        Self { inner }
    }
}

/// Internal macro to generate punctuated wrapper types.
///
/// Each wrapper shares the same structure but differs in:
/// - `POLICY` constant
/// - `trailing_punct()` behavior
/// - Documentation
macro_rules! impl_punctuated_wrapper {
    (
        $(#[$attr:meta])*
        $name:ident,
        $policy:expr,
        $trailing_punct:expr
    ) => {
        $(#[$attr])*
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[derive(Debug, Clone)]
        pub struct $name<T, P>(PunctuatedInner<T, P>);

        impl<T, P> $name<T, P> {
            /// The trailing punctuation policy for this type.
            pub const POLICY: TrailingPolicy = $policy;

            /// Creates a new empty sequence.
            #[inline]
            pub fn new() -> Self {
                Self(PunctuatedInner::new())
            }

            /// Creates a new sequence with pre-allocated capacity.
            #[inline]
            pub fn with_capacity(capacity: usize) -> Self {
                Self(PunctuatedInner::with_capacity(capacity))
            }

            /// Pushes a value onto the sequence.
            #[inline]
            pub fn push_value(&mut self, value: T) {
                self.0.push_value(value);
            }

            /// Attaches punctuation to the last value.
            #[inline]
            pub fn push_punct(&mut self, punct: P) {
                self.0.push_punct(punct);
            }

            /// Returns whether the sequence has trailing punctuation.
            ///
            /// The return value depends on the wrapper type's policy:
            /// - `Punctuated`: delegates to inner storage
            /// - `Terminated`: always `true`
            /// - `Separated`: always `false`
            #[inline]
            pub fn trailing_punct(&self) -> bool {
                $trailing_punct(&self.0)
            }

            /// Consumes the wrapper and returns the inner storage.
            #[inline]
            pub fn into_inner(self) -> PunctuatedInner<T, P> {
                self.0
            }
        }

        impl<T, P> Default for $name<T, P> {
            fn default() -> Self {
                Self::new()
            }
        }

        impl<T, P> IntoIterator for $name<T, P> {
            type Item = T;
            type IntoIter = std::iter::Map<std::vec::IntoIter<(T, Option<P>)>, fn((T, Option<P>)) -> T>;

            fn into_iter(self) -> Self::IntoIter {
                self.0.inner.into_iter().map(|(v, _)| v)
            }
        }

        impl<T, P> FromIterator<T> for $name<T, P> {
            fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
                Self(PunctuatedInner::from_iter(iter))
            }
        }

        impl<T, P> std::ops::Deref for $name<T, P> {
            type Target = PunctuatedInner<T, P>;

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<T, P> std::ops::DerefMut for $name<T, P> {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl<T, P> AsRef<PunctuatedInner<T, P>> for $name<T, P> {
            #[inline]
            fn as_ref(&self) -> &PunctuatedInner<T, P> {
                &self.0
            }
        }

        impl<T, P> AsMut<PunctuatedInner<T, P>> for $name<T, P> {
            #[inline]
            fn as_mut(&mut self) -> &mut PunctuatedInner<T, P> {
                &mut self.0
            }
        }
    };
}

impl_punctuated_wrapper!(
    /// A punctuated sequence with optional trailing separator.
    ///
    /// Use this for lists where trailing punctuation is allowed but not required,
    /// such as array literals: `[1, 2, 3]` or `[1, 2, 3,]`.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The value type (e.g., expression)
    /// - `P`: The punctuation/separator type (e.g., comma token)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Parse: item, item, item
    /// let mut items = Punctuated::<Item, CommaToken>::new();
    /// items.push_value(stream.parse()?);
    /// while stream.peek::<CommaToken>() {
    ///     items.push_punct(stream.parse()?);
    ///     if !stream.peek::<Item>() { break; } // Allow trailing comma
    ///     items.push_value(stream.parse()?);
    /// }
    /// ```
    Punctuated,
    TrailingPolicy::Optional,
    |inner: &PunctuatedInner<T, P>| inner.trailing_punct()
);

impl_punctuated_wrapper!(
    /// A punctuated sequence with required trailing separator.
    ///
    /// Use this for statement-like constructs where each item must end with
    /// punctuation, such as `use` statements: `use foo; use bar;`.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The value type (e.g., statement)
    /// - `P`: The punctuation/separator type (e.g., semicolon token)
    Terminated,
    TrailingPolicy::Required,
    |_: &PunctuatedInner<T, P>| true
);

impl_punctuated_wrapper!(
    /// A punctuated sequence where trailing separator is forbidden.
    ///
    /// Use this for function arguments or similar constructs where trailing
    /// punctuation is invalid: `f(a, b, c)` not `f(a, b, c,)`.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The value type (e.g., argument)
    /// - `P`: The punctuation/separator type (e.g., comma token)
    Separated,
    TrailingPolicy::Forbidden,
    |_: &PunctuatedInner<T, P>| false
);
