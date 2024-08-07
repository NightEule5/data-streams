// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "alloc")]

use alloc::{collections::VecDeque, vec::Vec};
#[cfg(feature = "utf8")]
use simdutf8::compat::from_utf8;
#[cfg(feature = "utf8")]
use crate::Error;
use crate::{BufferAccess, DataSink, DataSource, Result};

impl DataSink for Vec<u8> {
	fn write_bytes(&mut self, buf: &[u8]) -> Result {
		self.try_reserve(buf.len())?;
		self.extend_from_slice(buf);
		Ok(())
	}

	fn write_u8(&mut self, value: u8) -> Result {
		self.try_reserve(1)?;
		self.push(value);
		Ok(())
	}

	fn write_i8(&mut self, value: i8) -> Result {
		self.write_u8(value as u8)
	}
}

impl DataSource for VecDeque<u8> {
	fn available(&self) -> usize { self.len() }

	fn request(&mut self, count: usize) -> Result<bool> {
		Ok(self.len() >= count)
	}

	fn skip(&mut self, mut count: usize) -> Result<usize> {
		count = count.min(self.len());
		self.drain_buffer(count);
		Ok(count)
	}

	fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		let (mut a, mut b) = self.as_slices();
		let mut slice = &mut *buf;
		let mut count = a.read_bytes(slice)?.len();
		slice = &mut slice[count..];
		count += b.read_bytes(slice)?.len();
		self.drain_buffer(count);
		Ok(&buf[..count])
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
		match self.as_slices() {
			(mut bytes, _) if bytes.len() >= buf.len() => {
				// The deque is contiguous up to the buffer length, validate its
				// data in one go.
				let len = bytes.len();
				let result = bytes.read_utf8(buf);
				let consumed = len - bytes.len();
				self.drain_buffer(consumed);
				result
			}
			(mut a, mut b) => {
				// The deque is discontiguous. Validate the first slice, then the
				// second. If the first slice has an incomplete char, attempt to
				// rotate it into the second slice before proceeding.

				let mut slice = &mut *buf;

				let offset = match a.read_utf8(slice) {
					Ok(str) => str.len(),
					Err(Error::Utf8(error)) if error.error_len().is_none() => {
						// Incomplete char. Check if the char is completed on the
						// second slice, then rotate such that the second slice
						// contains the completed char.
						let char_start = error.valid_up_to();
						let incomplete = a.len() - char_start;
						let width = utf8_char_width(a[char_start]);
						let remaining = width - incomplete;

						if b.len() < remaining {
							// The char is actually incomplete. Consume the valid
							// bytes, then return the error.
							self.drain_buffer(char_start);
							return Err(error.into())
						}

						self.rotate_right(incomplete);
						(a, b) = self.as_slices();
						assert_eq!(a.len(), char_start);
						char_start
					}
					Err(error @ Error::Utf8(_)) =>
						// Invalid bytes, this error is unrecoverable.
						return Err(error),
					Err(_) => unreachable!() // <[u8]>::read_utf8 only ever returns Error::Utf8.
				};
				slice = &mut slice[offset..];

				match b.read_utf8(slice) {
					Ok(str) => {
						let valid = offset + str.len();
						self.drain_buffer(valid);
						Ok(unsafe {
							// Safety: these bytes have been validated as UTF-8 up
							// this point.
							core::str::from_utf8_unchecked(&buf[..valid])
						})
					}
					Err(Error::Utf8(mut error)) => {
						error.set_offset(offset);
						self.drain_buffer(error.valid_up_to());
						Err(Error::Utf8(error))
					}
					Err(_) => unreachable!() // <[u8]>::read_utf8 only ever returns Error::Utf8.
				}
			}
		}
	}
}

impl BufferAccess for VecDeque<u8> {
	fn buffer_capacity(&self) -> usize { self.capacity() }

	fn buffer(&self) -> &[u8] { self.as_slices().0 }

	fn fill_buffer(&mut self) -> Result<&[u8]> {
		Ok((*self).buffer()) // Nothing to read
	}

	fn clear_buffer(&mut self) {
		self.clear();
	}

	fn drain_buffer(&mut self, count: usize) {
		if self.len() == count {
			self.clear();
		} else {
			self.drain(..count);
		}
	}
}

impl DataSink for VecDeque<u8> {
	fn write_bytes(&mut self, buf: &[u8]) -> Result {
		self.try_reserve(buf.len())?;
		self.extend(buf);
		Ok(())
	}

	fn write_u8(&mut self, value: u8) -> Result {
		self.try_reserve(1)?;
		self.push_back(value);
		Ok(())
	}

	fn write_i8(&mut self, value: i8) -> Result {
		self.write_u8(value as u8)
	}
}

#[cfg(feature = "utf8")]
impl DataSink for alloc::string::String {
	fn write_bytes(&mut self, buf: &[u8]) -> Result {
		self.push_str(from_utf8(buf)?);
		Ok(())
	}
}

/// A reimplementation of the unstable [`core::str::utf8_char_width`] function.
fn utf8_char_width(byte: u8) -> usize {
	const UTF8_CHAR_WIDTH: &[u8; 256] = &[
		// 1  2  3  4  5  6  7  8  9  A  B  C  D  E  F
		1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 0
		1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 1
		1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 2
		1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 3
		1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 4
		1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 5
		1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 6
		1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 7
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 8
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 9
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // A
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // B
		0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // C
		2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // D
		3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // E
		4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // F
	];
	
	UTF8_CHAR_WIDTH[byte as usize] as usize
}
