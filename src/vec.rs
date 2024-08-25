// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "alloc")]

use alloc::{collections::VecDeque, vec::Vec};
#[cfg(feature = "utf8")]
use core::mem::MaybeUninit;
#[cfg(feature = "utf8")]
use simdutf8::compat::from_utf8;
#[cfg(any(feature = "utf8", feature = "unstable_ascii_char"))]
use crate::Error;
use crate::{BufferAccess, DataSink, DataSource, Result};
use crate::markers::source::SourceSize;
use crate::source::VecSource;
#[cfg(feature = "utf8")]
use crate::utf8::utf8_char_width;

impl DataSink for Vec<u8> {
	fn write_bytes(&mut self, buf: &[u8]) -> Result {
		self.try_reserve(buf.len())?;
		self.extend_from_slice(buf);
		Ok(())
	}

	fn write_utf8_codepoint(&mut self, value: char) -> Result {
		let start = self.len();
		let width = value.len_utf8();
		self.try_reserve(width)?;
		self.resize(start + width, 0);
		value.encode_utf8(&mut self[start..]);
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
	/// Reads bytes into a slice, returning them as an ASCII slice if valid.
	///
	/// # Errors
	///
	/// Returns [`Error::Ascii`] if a non-ASCII byte is found. This implementation
	/// consumes only valid ASCII. `buf` is left with valid ASCII bytes with a
	/// length of [`AsciiError::valid_up_to`]. The valid slice can be retrieved
	/// with [`AsciiError::valid_slice`].
	#[cfg(feature = "unstable_ascii_char")]
	fn read_ascii<'a>(&mut self, mut buf: &'a mut [u8]) -> Result<&'a [core::ascii::Char]> {
		use crate::source::count_ascii;

		let buf_len = self.len().min(buf.len());
		buf = &mut buf[..buf_len];
		let (mut a, mut b) = self.as_slices();
		if buf.len() >= a.len() {
			b = &b[..buf.len() - a.len()];
		} else {
			a = &a[..buf.len()];
			b = &[];
		}
		
		let a_count = count_ascii(a);
		if a_count == a.len() {
			buf.copy_from_slice(a);
			let b_count = count_ascii(b);
			buf[a_count..][..b_count].copy_from_slice(&b[..b_count]);
			
			let result = if b_count == b.len() {
				// Safety: all data is valid ASCII.
				Ok(unsafe { buf.as_ascii_unchecked() })
			} else {
				Err(Error::invalid_ascii(b[b_count], b_count, b_count))
			};
			self.drain_buffer(a_count + b_count);
			result
		} else {
			buf[..a_count].copy_from_slice(&a[..a_count]);
			self.drain_buffer(a_count);
			Err(Error::invalid_ascii(buf[a_count], a_count, a_count))
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

impl VecSource for VecDeque<u8> {
	fn read_to_end<'a>(&mut self, buf: &'a mut Vec<u8>) -> Result<&'a [u8]> {
		let start = buf.len();
		buf.extend(core::mem::take(self));
		Ok(&buf[start..])
	}

	#[cfg(feature = "utf8")]
	fn read_utf8_to_end<'a>(&mut self, buf: &'a mut alloc::string::String) -> Result<&'a str> {
		let start_len = buf.len();
		buf.try_reserve(self.len())?;
		unsafe {
			// Safety: the existing contents are not changed, and when this block
			// ends the buffer will have been checked as valid UTF-8.
			let buf = buf.as_mut_vec();
			
			let slice = {
				let spare = &mut buf.spare_capacity_mut()[..self.len()];
				spare.fill(MaybeUninit::new(0));
				// Safety: read_utf8 does not read from the buffer, and the returned
				// slice is guaranteed to be initialized.
				&mut *(core::ptr::from_mut::<[MaybeUninit<u8>]>(spare) as *mut [u8])
			};
			
			let result = self.read_utf8(slice);
			let valid_len = match result.as_ref() {
				Ok(valid) => valid.len(),
				Err(Error::Utf8(error)) => error.valid_up_to(),
				Err(_) => unreachable!() // read_utf8 only returns Error::Utf8.
			};
			// Safety: these bytes are initialized and valid UTF-8.
			buf.set_len(start_len + valid_len);
		}
		self.clear();
		Ok(&buf[start_len..])
	}
}

unsafe impl SourceSize for VecDeque<u8> {
	fn lower_bound(&self) -> u64 { self.len() as u64 }
	fn upper_bound(&self) -> Option<u64> { Some(self.len() as u64) }
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
	/// Writes all valid UTF-8 bytes from `buf`.
	///
	/// # Errors
	///
	/// Returns [`Error::Utf8`] if `buf` contains invalid UTF-8. In this case, any
	/// valid UTF-8 is written. [`Utf8Error::valid_up_to`] in this error returns
	/// the number of valid bytes written to the string.
	///
	/// [`Error::Allocation`] is returned when capacity cannot be allocated.
	fn write_bytes(&mut self, buf: &[u8]) -> Result {
		let (valid, result) = match from_utf8(buf).map_err(crate::Utf8Error::from) {
			Ok(str) => (str, Ok(())),
			Err(err) =>
				// Safety: this is safe because we use the same slice passed to the
				// validator. 
				(unsafe { err.valid_slice_unchecked(buf) }, Err(err.into()))
		};
		self.write_utf8(valid)?;
		result
	}
	/// Writes a UTF-8 string.
	///
	/// # Errors
	///
	/// [`Error::Allocation`] is returned when capacity cannot be allocated.
	fn write_utf8(&mut self, value: &str) -> Result {
		self.try_reserve(value.len())?;
		self.push_str(value);
		Ok(())
	}
	/// Writes a single UTF-8 codepoint.
	/// 
	/// # Errors
	/// 
	/// [`Error::Allocation`] is returned when capacity cannot be allocated.
	fn write_utf8_codepoint(&mut self, value: char) -> Result {
		self.try_reserve(value.len_utf8())?;
		self.push(value);
		Ok(())
	}
}
