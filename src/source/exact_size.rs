// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

use std::ops::Deref;
#[cfg(feature = "utf8")]
use simdutf8::compat::from_utf8;
use crate::{BufferAccess, DataSource, Result};

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

// How does adding the `+ BufferAccess` bound solve the conflicting implementation?
impl<T: ExactSizeBuffer + BufferAccess> DataSource for T {
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
}

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
