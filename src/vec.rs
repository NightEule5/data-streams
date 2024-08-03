// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "alloc")]

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::str::from_utf8_unchecked;
use simdutf8::compat::from_utf8;
use crate::{BufferAccess, DataSink, DataSource, Result};

impl DataSource for Vec<u8> {
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
		let slice = self.as_slice().read_bytes(buf)?;
		self.consume(slice.len());
		Ok(slice)
	}

	fn read_utf8<'a>(&mut self, mut count: usize, buf: &'a mut String) -> Result<&'a str> {
		if buf.is_empty() && count >= self.len() {
			// If the string is empty and all bytes are being read, we can avoid
			// copying by replacing the buffer with the vec.
			from_utf8(self)?;
			// Todo: This can be replaced with a SIMD version of String::from_utf8
			//  when simdutf8#73 is implemented.
			*buf = unsafe {
				String::from_utf8_unchecked(core::mem::take(self))
			};
			return Ok(buf)
		}

		count = count.min(self.len());
		let bytes = &self[..count];
		let start_len = buf.len();
		buf.push_str(from_utf8(bytes)?);
		self.consume(count);
		Ok(&buf[start_len..])
	}

	fn read_utf8_to_end<'a>(&mut self, buf: &'a mut String) -> Result<&'a str> {
		self.read_utf8(self.len(), buf)
	}
}

impl BufferAccess for Vec<u8> {
	fn buf_capacity(&self) -> usize { self.capacity() }

	fn buf(&self) -> &[u8] { self }

	fn fill_buf(&mut self) -> Result<&[u8]> {
		Ok(self) // Nothing to read
	}

	fn clear_buf(&mut self) {
		self.clear()
	}

	fn consume(&mut self, count: usize) {
		if count == self.len() {
			self.clear();
		} else {
			self.drain(..count);
		}
	}
}

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
		self.consume(count);
		Ok(count)
	}

	fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		let (mut a, mut b) = self.as_slices();
		let mut slice = &mut *buf;
		let mut count = a.read_bytes(slice)?.len();
		slice = &mut slice[count..];
		count += b.read_bytes(slice)?.len();
		self.consume(count);
		Ok(&buf[..count])
	}

	fn read_utf8<'a>(&mut self, mut count: usize, buf: &'a mut String) -> Result<&'a str> {
		count = count.min(self.len());
		let start_len = buf.len();
		match self.as_slices() {
			(bytes, &[]) => {
				// The deque is contiguous, validate its data in one go.
				let bytes = &bytes[..count];
				buf.push_str(from_utf8(bytes)?);
				self.consume(count);
			}
			(mut a, mut b) => {
				// The deque is discontiguous. Validate the first slice, then the
				// second. If the first slice has an incomplete char, attempt to
				// rotate it into the second slice before proceeding.
				
				let str_a = match from_utf8(a) {
					Ok(str) => str,
					Err(error) if error.error_len().is_none() => {
						// Incomplete char. Check if the char is completed on the
						// second slice, then rotate such that the second slice
						// contains the completed char.
						let char_start = error.valid_up_to();
						let incomplete = a.len() - char_start;
						let width = utf8_char_width(a[char_start]);
						let remaining = width - incomplete;
						
						if b.len() < remaining {
							return Err(error.into())
						}
						
						self.rotate_right(incomplete);
						(a, b) = self.as_slices();
						assert_eq!(a.len(), char_start);
						unsafe {
							// Safety: this slice has been checked to contain valid
							// UTF-8.
							from_utf8_unchecked(a)
						}
					}
					Err(error) => return Err(error.into())
				};
				let str_b = from_utf8(b)?;
				buf.try_reserve(str_a.len() + str_b.len())?;
				buf.push_str(str_a);
				buf.push_str(str_b);
			}
		}
		Ok(&buf[start_len..])
	}

	fn read_utf8_to_end<'a>(&mut self, buf: &'a mut String) -> Result<&'a str> {
		self.read_utf8(self.len(), buf)
	}
}

impl BufferAccess for VecDeque<u8> {
	fn buf_capacity(&self) -> usize { self.capacity() }

	fn buf(&self) -> &[u8] { self.as_slices().0 }

	fn fill_buf(&mut self) -> Result<&[u8]> {
		Ok((*self).buf()) // Nothing to read
	}

	fn clear_buf(&mut self) {
		self.clear()
	}

	fn consume(&mut self, count: usize) {
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

impl DataSink for String {
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

