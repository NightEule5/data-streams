// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "alloc")]
use alloc::collections::TryReserveError;
#[cfg(feature = "unstable_ascii_char")]
use core::ascii;
#[cfg(feature = "utf8")]
pub use simdutf8::compat::Utf8Error as SimdUtf8Error;
use core::fmt::{Display, Formatter, Result as FmtResult};
#[cfg(feature = "utf8")]
use core::num::NonZeroU8;

/// A stream error.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
	/// An IO error.
	#[cfg(feature = "std")]
	Io(std::io::Error),
	/// An invalid ASCII byte was encountered.
	#[cfg(feature = "unstable_ascii_char")]
	Ascii(AsciiError),
	/// Invalid UTF-8 bytes were encountered.
	#[cfg(feature = "utf8")]
	Utf8(Utf8Error),
	/// Error while attempting to reserve capacity.
	#[cfg(feature = "alloc")]
	Allocation(TryReserveError),
	/// A sink reached a hard storage limit, causing an overflow while writing. An
	/// example is a mutable slice, which can't write more bytes than its length.
	Overflow {
		/// The byte count remaining in the attempted read operation.
		remaining: usize
	},
	/// Premature end-of-stream.
	End {
		/// The total required byte count.
		required_count: usize
	},
	/// A "read to end" method was called on a source with no defined end.
	NoEnd,
	/// Buffer size is insufficient to buffer a read operation.
	InsufficientBuffer {
		/// The buffer's spare capacity.
		spare_capacity: usize,
		/// The total required byte count.
		required_count: usize
	},
}

impl Error {
	/// Creates an ASCII error.
	#[inline]
	#[cfg(feature = "unstable_ascii_char")]
	pub const fn invalid_ascii(invalid_byte: u8, valid_up_to: usize, consumed_count: usize) -> Self {
		assert!(consumed_count >= valid_up_to);
		Self::Ascii(AsciiError { invalid_byte, valid_up_to, consumed_count })
	}
	/// Creates an overflow error.
	#[inline]
	pub const fn overflow(remaining: usize) -> Self {
		Self::Overflow { remaining }
	}
	/// Creates an end-of-stream error.
	#[inline]
	pub const fn end(required_count: usize) -> Self {
		Self::End { required_count }
	}
	/// Creates an insufficient buffer capacity error.
	#[inline]
	pub const fn insufficient_buffer(spare_capacity: usize, required_count: usize) -> Self {
		Self::InsufficientBuffer { spare_capacity, required_count }
	}
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Io(error) => Some(error),
			#[cfg(feature = "unstable_ascii_char")]
			Self::Ascii(_) => None,
			#[cfg(feature = "utf8")]
			Self::Utf8(error) => error.source(),
			#[cfg(feature = "alloc")]
			Self::Allocation(error) => Some(error),
			Self::Overflow { .. } |
			Self::End { .. } |
			Self::NoEnd |
			Self::InsufficientBuffer { .. } => None,
		}
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			#[cfg(feature = "std")]
			Self::Io(error) => Display::fmt(error, f),
			#[cfg(feature = "unstable_ascii_char")]
			Self::Ascii(error) => Display::fmt(error, f),
			#[cfg(feature = "utf8")]
			Self::Utf8(error) => Display::fmt(error, f),
			#[cfg(feature = "alloc")]
			Self::Allocation(error) => Display::fmt(error, f),
			Self::Overflow { remaining } => write!(f, "sink overflowed with {remaining} bytes remaining to write"),
			Self::End { required_count } => write!(f, "premature end-of-stream when reading {required_count} bytes"),
			Self::NoEnd => write!(f, "cannot read to end of infinite source"),
			Self::InsufficientBuffer {
				spare_capacity, required_count
			} => write!(f, "insufficient buffer capacity ({spare_capacity}) to read {required_count} bytes"),
		}
	}
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
	#[inline]
	fn from(value: std::io::Error) -> Self {
		Self::Io(value)
	}
}

#[cfg(feature = "utf8")]
impl From<SimdUtf8Error> for Error {
	#[inline]
	fn from(value: SimdUtf8Error) -> Self {
		Self::Utf8(value.into())
	}
}

#[cfg(feature = "utf8")]
impl From<Utf8Error> for Error {
	#[inline]
	fn from(value: Utf8Error) -> Self {
		Self::Utf8(value)
	}
}

#[cfg(feature = "unstable_ascii_char")]
impl From<AsciiError> for Error {
	#[inline]
	fn from(value: AsciiError) -> Self {
		Self::Ascii(value)
	}
}

#[cfg(feature = "alloc")]
impl From<TryReserveError> for Error {
	#[inline]
	fn from(value: TryReserveError) -> Self {
		Self::Allocation(value)
	}
}

#[cfg(feature = "utf8")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Utf8Error {
	offset: usize,
	inner: SimdUtf8Error,
}

/// A kind of UTF-8 error.
#[cfg(feature = "utf8")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Utf8ErrorKind {
	/// The end of the input was reached while reading a character.
	IncompleteChar,
	/// The next `1` to `4` bytes are invalid.
	InvalidBytes(NonZeroU8),
}

#[cfg(feature = "utf8")]
impl Utf8Error {
	/// Returns the index in the input to which valid UTF-8 was verified before the
	/// last error.
	#[inline]
	#[must_use]
	pub const fn offset(&self) -> usize { self.offset }
	/// Returns the index in the input to which valid UTF-8 was verified.
	#[inline]
	#[must_use]
	pub fn valid_up_to(&self) -> usize {
		self.offset + self.inner.valid_up_to()
	}
	/// Returns the length, in range `1..=3`, of the invalid byte sequence, if any.
	/// Reading may continue with these removed. If `None` is returned, an incomplete
	/// character sequence was encountered. This could be a valid character whose
	/// sequence spans multiple buffer chunks.
	#[inline]
	#[must_use]
	pub fn error_len(&self) -> Option<usize> {
		self.inner.error_len()
	}
	/// Returns the last [`Utf8Error`](SimdUtf8Error) without the offset. Calling
	/// [`valid_up_to`] may be meaningless, because multiple UTF-8 validations may
	/// have taken place while reading.
	/// 
	/// [`valid_up_to`]: SimdUtf8Error::valid_up_to
	#[inline]
	#[must_use]
	pub const fn last_error(&self) -> SimdUtf8Error { self.inner }
	/// Returns the kind of error encountered.
	#[inline]
	#[must_use]
	pub fn error_kind(&self) -> Utf8ErrorKind {
		match self.inner.error_len() {
			Some(len) => Utf8ErrorKind::InvalidBytes(unsafe {
				// Safety: core::str::from_utf8 (used by simdutf8 to get the error)
				// never returns an error_len outside the range 1..=3, so the cast
				// never truncates and conversion to non-zero is safe.
				NonZeroU8::new_unchecked(len as u8)
			}),
			None => Utf8ErrorKind::IncompleteChar
		}
	}
	/// Splits a slice at the valid UTF-8 index, returning the first slice as a
	/// string.
	/// 
	/// # Safety
	/// 
	/// The caller promises the slice has exactly the same contents and length as
	/// the slice passed to the method which produced the error. Passing another
	/// slice may cause undefined behavior, such as the string containing invalid
	/// UTF-8, or reading out-of-bounds if the slice is shorter than the valid
	/// length.
	pub unsafe fn split_valid<'a>(&self, bytes: &'a [u8]) -> (&'a str, &'a [u8]) {
		let (valid, invalid) = bytes.split_at_unchecked(self.valid_up_to());
		(core::str::from_utf8_unchecked(valid), invalid)
	}
	/// Splits a mutable slice at the valid UTF-8 index, returning the first slice
	/// as a string.
	/// 
	/// # Safety
	///
	/// The caller promises the slice has exactly the same contents and length as
	/// the slice passed to the method which produced the error. Passing another
	/// slice may cause undefined behavior, such as the string containing invalid
	/// UTF-8, or reading out-of-bounds if the slice is shorter than the valid
	/// length.
	pub unsafe fn split_valid_mut<'a>(&self, bytes: &'a mut [u8]) -> (&'a mut str, &'a mut [u8]) {
		let (valid, invalid) = bytes.split_at_mut_unchecked(self.valid_up_to());
		(core::str::from_utf8_unchecked_mut(valid), invalid)
	}
}

#[cfg(feature = "utf8")]
impl Utf8Error {
	#[cfg(any(feature = "unstable_specialization", feature = "alloc"))]
	pub(crate) fn set_offset(&mut self, offset: usize) {
		self.offset += offset;
	}
	#[cfg(feature = "unstable_specialization")]
	pub(crate) fn with_offset(mut self, offset: usize) -> Self {
		self.set_offset(offset);
		self
	}
}

#[cfg(all(feature = "std", feature = "utf8"))]
impl std::error::Error for Utf8Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		Some(&self.inner)
	}
}

#[cfg(feature = "utf8")]
impl Display for Utf8Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let valid_up_to = self.valid_up_to();
		match self.error_len() {
			Some(len) => write!(f, "invalid UTF-8 sequence of {len} bytes from index {valid_up_to}"),
			None => write!(f, "incomplete UTF-8 byte sequence from index {valid_up_to}")
		}
	}
}

#[cfg(feature = "utf8")]
impl From<SimdUtf8Error> for Utf8Error {
	#[inline]
	fn from(inner: SimdUtf8Error) -> Self {
		Self { offset: 0, inner }
	}
}

#[cfg(feature = "unstable_ascii_char")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct AsciiError {
	/// The invalid byte, in range `128..256`.
	pub invalid_byte: u8,
	/// The index of the invalid byte to which the input was valid ASCII.
	pub valid_up_to: usize,
	/// The number of bytes consumed from the source, including any unchecked bytes
	/// after the invalid byte.
	pub consumed_count: usize,
}

#[cfg(feature = "unstable_ascii_char")]
impl AsciiError {
	/// Returns the invalid byte.
	#[inline]
	#[must_use]
	pub const fn invalid_byte(&self) -> u8 { self.invalid_byte }
	/// Returns the index in the input to which valid ASCII was verified.
	#[inline]
	#[must_use]
	pub const fn valid_up_to(&self) -> usize { self.valid_up_to }
	/// Returns the total number of bytes consumed from the source, including any
	/// unchecked bytes after the invalid byte.
	#[inline]
	#[must_use]
	pub const fn consumed_count(&self) -> usize { self.consumed_count }
	/// Returns the number of bytes consumed from the source after the invalid byte
	/// which haven't been checked.
	#[inline]
	#[must_use]
	pub const fn unchecked_count(&self) -> usize { self.consumed_count.saturating_sub(1 + self.valid_up_to) }
	/// Returns the validated part of a slice as ASCII.
	/// 
	/// # Panics
	/// 
	/// Panics if the slice does not contain valid bytes up to the valid length in
	/// the error.
	#[must_use]
	pub fn valid_slice<'a>(&self, bytes: &'a [u8]) -> &'a [ascii::Char] {
		assert!(bytes.len() >= self.valid_up_to);
		assert!(bytes[..self.valid_up_to].is_ascii());
		unsafe {
			// Safety: the invariants were checked by the above assertions.
			self.valid_slice_unchecked(bytes)
		}
	}
	/// Returns the validated part of a slice as ASCII, assuming it has identical
	/// contents from the slice which produced the error.
	/// 
	/// # Safety
	/// 
	/// The slice length and contents must be identical to the slice which produced
	/// the error. Passing a shorter and/or unvalidated slice may cause UB, because
	/// it may index out-of-bounds or invalidate the result.
	/// 
	/// For a safe alternative, use [`valid_slice`](Self::valid_slice).
	#[must_use]
	pub unsafe fn valid_slice_unchecked<'a>(&self, bytes: &'a [u8]) -> &'a [ascii::Char] {
		bytes.get_unchecked(..self.valid_up_to).as_ascii_unchecked()
	}
	/// Splits a slice at the valid ASCII index, returning the first slice as an
	/// [`ascii::Char`] slice.
	/// 
	/// # Panics
	/// 
	/// Panics if the slice does not contain valid bytes up to the valid length in
	/// the error, or if shorter than the consumed count.
	#[must_use]
	pub fn split_valid<'a>(&self, bytes: &'a [u8]) -> (&'a [ascii::Char], &'a [u8]) {
		assert!(self.consumed_count >= self.valid_up_to);
		assert!(bytes.len() >= self.consumed_count);
		assert!(bytes[..self.valid_up_to].is_ascii());
		unsafe {
			// Safety: the invariants were checked by the above assertions.
			self.split_valid_unchecked(bytes)
		}
	}
	/// Splits a slice at the valid ASCII index, returning the first slice as an
	/// [`ascii::Char`] slice. Assumes the slice has identical contents from the
	/// slice which produced the error.
	///
	/// # Safety
	///
	/// The slice length and contents must be identical to the slice which produced
	/// the error. Passing a shorter and/or unvalidated slice may cause UB, because
	/// it may index out-of-bounds or invalidate the result.
	/// 
	/// For a safe alternative, use [`split_valid`](Self::split_valid).
	#[must_use]
	pub unsafe fn split_valid_unchecked<'a>(&self, bytes: &'a [u8]) -> (&'a [ascii::Char], &'a [u8]) {
		(self.valid_slice_unchecked(bytes),
		 bytes.get_unchecked(self.valid_up_to..self.consumed_count))
	}
}

#[cfg(all(feature = "std", feature = "unstable_ascii_char"))]
impl std::error::Error for AsciiError { }

#[cfg(feature = "unstable_ascii_char")]
impl Display for AsciiError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let Self { invalid_byte, valid_up_to, .. } = self;
		write!(f, "non-ASCII byte {invalid_byte:#X} at index {valid_up_to}")
	}
}
