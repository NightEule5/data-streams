// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "alloc")]
use alloc::collections::TryReserveError;
#[cfg(feature = "alloc")]
use simdutf8::compat::Utf8Error;
use core::fmt::{Display, Formatter, Result as FmtResult};

/// A stream error.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
	/// An IO error.
	#[cfg(feature = "std")]
	Io(std::io::Error),
	/// An invalid ASCII byte was encountered.
	#[cfg(feature = "alloc")]
	Ascii(u8),
	/// Invalid UTF-8 bytes were encountered.
	#[cfg(feature = "alloc")]
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
	/// Create an overflow error.
	#[inline]
	pub const fn overflow(remaining: usize) -> Self {
		Self::Overflow { remaining }
	}
	/// Create an end-of-stream error.
	#[inline]
	pub const fn end(required_count: usize) -> Self {
		Self::End { required_count }
	}
	/// Create an insufficient buffer capacity error.
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
			#[cfg(feature = "alloc")]
			Self::Ascii(_) => None,
			#[cfg(feature = "alloc")]
			Self::Utf8(error) => Some(error),
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
			#[cfg(feature = "alloc")]
			Self::Ascii(byte) => write!(f, "cannot read non-ASCII byte {byte:#X} into a UTF-8 string"),
			#[cfg(feature = "alloc")]
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

#[cfg(feature = "alloc")]
impl From<Utf8Error> for Error {
	#[inline]
	fn from(value: Utf8Error) -> Self {
		Self::Utf8(value)
	}
}

#[cfg(feature = "alloc")]
impl From<TryReserveError> for Error {
	#[inline]
	fn from(value: TryReserveError) -> Self {
		Self::Allocation(value)
	}
}