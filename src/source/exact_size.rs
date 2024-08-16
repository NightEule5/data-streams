// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

use core::ops::Deref;
#[cfg(feature = "utf8")]
use simdutf8::compat::from_utf8;
use crate::{BufferAccess, DataSource, Result};
#[cfg(feature = "unstable_ascii_char")]
use crate::Error;
use crate::markers::source::SourceSize;

trait ExactSizeBuffer: Deref<Target = [u8]> {
	fn len(&self) -> usize { (**self).len() }
	fn consume(&mut self, count: usize);

	fn read_bytes_infallible<'a>(&mut self, buf: &'a mut [u8]) -> &'a [u8] {
		let len = self.len().min(buf.len());
		let filled = &mut buf[..len];
		filled.copy_from_slice(&self[..len]);
		self.consume(len);
		filled
	}
}

// Conflicting implementation with blanket impl, use a macro instead.
macro_rules! impl_source {
    ($($(#[$meta:meta])?$ty:ty);+) => {
		$(
		$(#[$meta])?
		impl DataSource for $ty {
			fn available(&self) -> usize { self.len() }
			fn request(&mut self, count: usize) -> Result<bool> {
				Ok(self.len() >= count)
			}
		
			fn skip(&mut self, mut count: usize) -> Result<usize> {
				count = count.min(self.len());
				self.consume(count);
				Ok(count)
			}
		
			fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
				Ok(self.read_bytes_infallible(buf))
			}
		
			/// Reads bytes into a slice, returning them as a UTF-8 string if valid.
			///
			/// # Errors
			///
			/// Returns [`Error::Utf8`] if invalid UTF-8 is read. This implementation only
			/// consumes valid UTF-8. `buf` is left with a valid UTF-8 string whose length
			/// is given by the error, [`Utf8Error::valid_up_to`]. This slice can be safely
			/// converted to a string with [`from_str_unchecked`] or [`Utf8Error::split_valid`].
			///
			/// [`Utf8Error::valid_up_to`]: simdutf8::compat::Utf8Error::valid_up_to
			/// [`from_str_unchecked`]: core::str::from_utf8_unchecked
			#[cfg(feature = "utf8")]
			fn read_utf8<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a str> {
				let count = buf.len().min(self.len());
				let filled = &mut buf[..count];
				filled.copy_from_slice(&self[..count]);
				let (result, consumed) = match from_utf8(filled) {
					Ok(str) => (Ok(str), count),
					Err(error) => (Err(error.into()), error.valid_up_to())
				};
				self.consume(consumed);
				result
			}

			/// Reads bytes into a slice, returning them as an ASCII slice if valid.
			///
			/// # Errors
			///
			/// Returns [`Error::Ascii`] if a non-ASCII byte is found. This
			/// implementation consumes only valid ASCII. `buf` is left with valid
			/// ASCII bytes with a length of [`AsciiError::valid_up_to`]. The valid
			/// slice can be retrieved with [`AsciiError::valid_slice`].
			#[cfg(feature = "unstable_ascii_char")]
			fn read_ascii<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [core::ascii::Char]> {
				let len = buf.len().min(self.len());
				let count = super::count_ascii(&self[..len]);
				let bytes = self.read_bytes_infallible(&mut buf[..len]);
				if count == len {
					// Safety: all bytes have been checked as valid ASCII.
					Ok(unsafe { bytes.as_ascii_unchecked() })
				} else {
					Err(Error::invalid_ascii(bytes[count], count, count))
				}
			}
		})+
	};
}

impl_source! { &[u8]; #[cfg(feature = "alloc")] alloc::vec::Vec<u8> }

impl ExactSizeBuffer for &[u8] {
	fn consume(&mut self, count: usize) {
		*self = &self[..count];
	}
}

// Blanket impls break everything here.
impl BufferAccess for &[u8] {
	fn buffer_capacity(&self) -> usize { self.len() }

	fn buffer(&self) -> &[u8] { self }

	fn fill_buffer(&mut self) -> Result<&[u8]> { Ok(self) }

	fn drain_buffer(&mut self, count: usize) { self.consume(count); }
}

unsafe impl SourceSize for &[u8] {
	fn lower_bound(&self) -> u64 { self.len() as u64 }
	fn upper_bound(&self) -> Option<u64> { Some(self.len() as u64) }
}

#[cfg(feature = "alloc")]
impl ExactSizeBuffer for alloc::vec::Vec<u8> {
	fn consume(&mut self, count: usize) {
		if self.len() == count {
			self.clear();
		} else {
			self.drain(..count);
		}
	}
}

#[cfg(feature = "alloc")]
impl BufferAccess for alloc::vec::Vec<u8> {
	fn buffer_capacity(&self) -> usize { self.capacity() }

	fn buffer(&self) -> &[u8] { self }

	fn fill_buffer(&mut self) -> Result<&[u8]> { Ok(self) }

	fn drain_buffer(&mut self, count: usize) { self.consume(count); }
}

#[cfg(feature = "alloc")]
unsafe impl SourceSize for alloc::vec::Vec<u8> {
	fn lower_bound(&self) -> u64 { self.len() as u64 }
	fn upper_bound(&self) -> Option<u64> { Some(self.len() as u64) }
}
