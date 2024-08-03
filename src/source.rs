// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "alloc")]
use alloc::string::String;
use bytemuck::{bytes_of_mut, Pod};
use num_traits::PrimInt;
use crate::{Error, Result, slice};

/// A source stream of data.
pub trait DataSource {
	/// Returns the number of bytes available for reading. This does not necessarily
	/// mean more data isn't available, just that *at least* this count is may be
	/// read.
	fn available(&self) -> usize;
	/// Reads at most `count` bytes into an internal buffer, returning whether
	/// enough bytes are available. To return an end-of-stream error, use [`require`]
	/// instead.
	///
	/// Note that a request returning `false` doesn't necessarily mean the stream
	/// has ended. More bytes may be read after.
	///
	/// # Errors
	///
	/// If the byte count exceeds the spare buffer capacity, [`Error::InsufficientBuffer`]
	/// is returned and both the internal buffer and underlying streams remain unchanged.
	///
	/// [`require`]: Self::require
	fn request(&mut self, count: usize) -> Result<bool>;
	/// Reads at least `count` bytes into an internal buffer, returning `Ok` if
	/// successful, or an end-of-stream error if not. For a softer version that
	/// returns whether enough bytes are available, use [`request`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ended before `count` bytes could be
	/// read. If the byte count exceeds the spare buffer capacity, [`Error::InsufficientBuffer`]
	/// is returned instead.
	///
	/// [`request`]: Self::request
	fn require(&mut self, count: usize) -> Result {
		if self.request(count)? {
			Ok(())
		} else {
			Err(Error::End { required_count: count })
		}
	}

	/// Consumes up to `count` bytes in the stream, returning the number of bytes
	/// consumed if successful. At least the available count may be consumed.
	fn skip(&mut self, count: usize) -> Result<usize>;
	/// Reads bytes into a slice, returning the bytes read. This method is greedy;
	/// it consumes as many bytes as it can, until `buf` is filled or no more bytes
	/// are read.
	fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]>;
	/// Reads the exact length of bytes into a slice, returning the bytes read if
	/// successful, or an end-of-stream error if not. Bytes are not consumed if an
	/// end-of-stream error is returned.
	fn read_exact_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		default_read_exact_bytes(self, buf)
	}
	/// Reads an array with a size of `N` bytes.
	fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> where Self: Sized {
		default_read_array(self)
	}

	/// Reads a [`u8`].
	fn read_u8(&mut self) -> Result<u8> { self.read_int_be_spec() }
	/// Reads an [`i8`].
	fn read_i8(&mut self) -> Result<i8> { self.read_int_be_spec() }
	/// Reads a big-endian [`u16`].
	fn read_u16(&mut self) -> Result<u16> { self.read_int_be_spec() }
	/// Reads a big-endian [`i16`].
	fn read_i16(&mut self) -> Result<i16> { self.read_int_be_spec() }
	/// Reads a little-endian [`u16`].
	fn read_u16_le(&mut self) -> Result<u16> { self.read_int_le_spec() }
	/// Reads a little-endian [`i16`].
	fn read_i16_le(&mut self) -> Result<i16> { self.read_int_le_spec() }
	/// Reads a big-endian [`u32`].
	fn read_u32(&mut self) -> Result<u32> { self.read_int_be_spec() }
	/// Reads a big-endian [`i32`].
	fn read_i32(&mut self) -> Result<i32> { self.read_int_be_spec() }
	/// Reads a little-endian [`u32`].
	fn read_u32_le(&mut self) -> Result<u32> { self.read_int_le_spec() }
	/// Reads a little-endian [`i32`].
	fn read_i32_le(&mut self) -> Result<i32> { self.read_int_le_spec() }
	/// Reads a big-endian [`u64`].
	fn read_u64(&mut self) -> Result<u64> { self.read_int_be_spec() }
	/// Reads a big-endian [`i64`].
	fn read_i64(&mut self) -> Result<i64> { self.read_int_be_spec() }
	/// Reads a little-endian [`u64`].
	fn read_u64_le(&mut self) -> Result<u64> { self.read_int_le_spec() }
	/// Reads a little-endian [`i64`].
	fn read_i64_le(&mut self) -> Result<i64> { self.read_int_le_spec() }
	/// Reads a big-endian [`u128`].
	fn read_u128(&mut self) -> Result<u128> { self.read_int_be_spec() }
	/// Reads a big-endian [`i128`].
	fn read_i128(&mut self) -> Result<i128> { self.read_int_be_spec() }
	/// Reads a little-endian [`u128`].
	fn read_u128_le(&mut self) -> Result<u128> { self.read_int_le_spec() }
	/// Reads a little-endian [`i128`].
	fn read_i128_le(&mut self) -> Result<i128> { self.read_int_le_spec() }
	/// Reads a big-endian [`usize`]. To make streams consistent across platforms,
	/// [`usize`] is fixed to the size of [`u64`] regardless of the target platform.
	fn read_usize(&mut self) -> Result<usize> {
		self.read_u64().map(|i| i as usize)
	}
	/// Reads a big-endian [`isize`]. To make streams consistent across platforms,
	/// [`isize`] is fixed to the size of [`i64`] regardless of the target platform.
	fn read_isize(&mut self) -> Result<isize> {
		self.read_i64().map(|i| i as isize)
	}
	/// Reads a little-endian [`usize`]. To make streams consistent across platforms,
	/// [`usize`] is fixed to the size of [`u64`] regardless of the target platform.
	fn read_usize_le(&mut self) -> Result<usize> {
		self.read_u64_le().map(|i| i as usize)
	}
	/// Reads a little-endian [`isize`]. To make streams consistent across platforms,
	/// [`isize`] is fixed to the size of [`i64`] regardless of the target platform.
	fn read_isize_le(&mut self) -> Result<isize> {
		self.read_i64_le().map(|i| i as isize)
	}

	/// Reads a big-endian integer.
	fn read_int<T: PrimInt + Pod>(&mut self) -> Result<T> where Self: Sized {
		self.read_int_be_spec()
	}
	/// Reads a little-endian integer.
	fn read_int_le<T: PrimInt + Pod>(&mut self) -> Result<T> where Self: Sized {
		self.read_int_le_spec()
	}

	/// Reads a value of generic type `T` supporting an arbitrary bit pattern. See
	/// [`Pod`].
	fn read_data<T: Pod>(&mut self) -> Result<T> where Self: Sized {
		self.read_data_spec()
	}

	/// Reads up to `count` bytes of UTF-8 into `buf`, returning the string read.
	/// If invalid bytes are encountered, an error is returned and `buf` is unchanged.
	/// In this case, the stream is left in a state with up to `count` bytes consumed
	/// from it, including the invalid bytes and any subsequent bytes.
	#[cfg(feature = "alloc")]
	fn read_utf8<'a>(&mut self, count: usize, buf: &'a mut String) -> Result<&'a str> {
		default_read_utf8(self, count, buf)
	}

	/// Reads UTF-8 bytes into `buf` until the end of the stream, returning the
	/// string read. If invalid bytes are encountered, an error is returned and
	/// `buf` is unchanged. In this case, the stream is left in a state with an
	/// undefined number of bytes read.
	#[cfg(feature = "alloc")]
	fn read_utf8_to_end<'a>(&mut self, buf: &'a mut String) -> Result<&'a str>;
}

/// Helper extension trait for reading generic data from an unsized source.
pub(crate) trait ReadSpec<T: Pod>: DataSource {
	fn read_int_be_spec(&mut self) -> Result<T> where T: PrimInt {
		self.read_data_spec().map(T::from_be)
	}
	fn read_int_le_spec(&mut self) -> Result<T> where T: PrimInt {
		self.read_data_spec().map(T::from_le)
	}
	fn read_data_spec(&mut self) -> Result<T> {
		let mut value = T::zeroed();
		self.read_exact_bytes(bytes_of_mut(&mut value))?;
		Ok(value)
	}
}

impl<S: DataSource + ?Sized, T: Pod> ReadSpec<T> for S { }

/// Accesses a source's internal buffer.
pub trait BufferAccess {
	/// Returns the capacity of the internal buffer.
	fn buf_capacity(&self) -> usize;
	/// Returns a slice over the filled portion of the internal buffer. This slice
	/// may not contain the whole buffer, for example if it can't be represented as
	/// just one slice.
	fn buf(&self) -> &[u8];
	/// Fills the internal buffer from the underlying stream, returning its contents
	/// if successful.
	fn fill_buf(&mut self) -> Result<&[u8]>;
	/// Clears the internal buffer.
	fn clear_buf(&mut self);
	/// Consumes `count` bytes from the internal buffer. The `count` must be `<=`
	/// the length of the slice returned by either [`buf`](Self::buf) or
	/// [`fill_buf`](Self::fill_buf)
	/// 
	/// # Panics
	/// 
	/// This method panics if `count` exceeds the buffer length.
	fn consume(&mut self, count: usize);
}

#[cfg(feature = "nightly_specialization")]
impl<T: BufferAccess + ?Sized> DataSource for T {
	default fn available(&self) -> usize {
		default_available(self)
	}

	default fn request(&mut self, count: usize) -> Result<bool> {
		default_request(self, count)
	}

	default fn skip(&mut self, count: usize) -> Result<usize> {
		default_skip(self, count)
	}

	default fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		let mut slice = &mut *buf;
		while !slice.is_empty() {
			let mut buf = match self.request(slice.len()) {
				Ok(_) => self.buf(),
				Err(Error::InsufficientBuffer { .. }) => self.fill_buf()?,
				Err(error) => return Err(error)
			};
			if buf.is_empty() {
				break
			}
			
			let count = buf.read_bytes(slice)?.len();
			slice = &mut slice[count..];
		}

		let unfilled = slice.len();
		let filled = buf.len() - unfilled;
		Ok(&buf[..filled])
	}

	default fn read_exact_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		buf_read_exact_bytes(self, buf)
	}

	#[cfg(feature = "alloc")]
	default fn read_utf8<'a>(&mut self, count: usize, buf: &'a mut String) -> Result<&'a str> {
		default_read_utf8(self, count, buf)
	}

	#[cfg(feature = "alloc")]
	default fn read_utf8_to_end<'a>(&mut self, _: &'a mut String) -> Result<&'a str> {
		todo!("will be removed later")
	}
}

pub(crate) fn default_available(source: &(impl BufferAccess + ?Sized)) -> usize {
	source.buf().len()
}

pub(crate) fn default_request(source: &mut (impl BufferAccess + DataSource + ?Sized), count: usize) -> Result<bool> {
	if source.available() < count {
		let buf_len = default_available(source);
		let spare_capacity = source.buf_capacity() - buf_len;
		if count < spare_capacity {
			Ok(source.fill_buf()?.len() >= count)
		} else {
			Err(Error::InsufficientBuffer {
				spare_capacity,
				required_count: count - buf_len,
			})
		}
	} else {
		Ok(true)
	}
}

pub(crate) fn default_skip(source: &mut (impl BufferAccess + DataSource + ?Sized), mut count: usize) -> Result<usize> {
	let avail = source.available();
	count = count.min(avail);
	source.consume(count);
	// Guard against faulty implementations by verifying that the buffered
	// bytes were removed.
	assert_eq!(source.available(), avail.saturating_sub(count));
	Ok(avail)
}

pub(crate) fn default_read_array<const N: usize>(source: &mut (impl DataSource + ?Sized)) -> Result<[u8; N]> {
	let mut array = [0; N];
	source.read_exact_bytes(&mut array)?;
	Ok(array)
}

fn try_read_exact_contiguous<'a>(source: &mut (impl DataSource + ?Sized), buf: &'a mut [u8]) -> Result<&'a [u8]> {
	let len = buf.len();
	let bytes = source.read_bytes(buf)?;
	assert_eq!(
		bytes.len(),
		len,
		"read_bytes should be greedy; at least {available} bytes were available \
		in the buffer, but only {read_len} bytes of the required {len} were read",
		available = source.available(),
		read_len = bytes.len()
	);
	Ok(bytes)
}

fn try_read_exact_discontiguous<'a>(
	source: &mut (impl DataSource + ?Sized),
	buf: &'a mut [u8],
	remaining: usize
) -> Result<&'a [u8]> {
	let filled = buf.len() - remaining;
	let read_count = source.read_bytes(&mut buf[..filled])?.len();
	if read_count < remaining {
		if source.available() < remaining {
			// Buffer was exhausted, meaning the stream ended prematurely
			Err(Error::End { required_count: buf.len() })
		} else {
			// read_bytes wasn't greedy, there were enough bytes in the buffer >:(
			panic!("read_bytes should have read {remaining} buffered bytes")
		}
	} else {
		// The whole slice has been confirmed to be filled.
		Ok(buf)
	}
}

fn default_read_exact_bytes<'a>(source: &mut (impl DataSource + ?Sized), buf: &'a mut [u8]) -> Result<&'a [u8]> {
	let len = buf.len();
	match source.require(len) {
		Ok(()) => try_read_exact_contiguous(source, buf),
		Err(Error::InsufficientBuffer { .. }) => {
			// The buffer is not large enough to read the slice contiguously, and
			// we have no access to the buffer to drain it. So just try reading and
			// check if all bytes were read.
			let remaining = buf.len();
			try_read_exact_discontiguous(source, buf, remaining)
		}
		Err(error) => Err(error)
	}
}

#[cfg(any(feature = "nightly_specialization", test))]
fn buf_read_exact_bytes<'a>(source: &mut (impl BufferAccess + ?Sized), buf: &'a mut [u8]) -> Result<&'a [u8]> {
	let len = buf.len();
	match source.require(len) {
		Ok(()) => try_read_exact_contiguous(source, buf),
		Err(Error::InsufficientBuffer { .. }) => {
			// We're doing a large read. Drain the internal buffer, then try reading.
			// Most default implementations of read_bytes optimize for this case by
			// skipping the buffer.

			let mut slice = &mut *buf;
			let mut s_buf = source.buf();
			while !slice.is_empty() && !s_buf.is_empty() {
				let len = slice::read_bytes_infallible(&mut s_buf, slice).len();
				slice = &mut slice[len..];
				source.consume(len);
				s_buf = source.buf();
			}

			let remaining = slice.len();
			try_read_exact_discontiguous(source, buf, remaining)
		}
		Err(error) => Err(error)
	}
}

#[cfg(feature = "alloc")]
pub(crate) fn default_read_utf8<'a>(
	source: &mut (impl DataSource + ?Sized),
	count: usize,
	buf: &'a mut String
) -> Result<&'a str> {
	buf.reserve(count);
	unsafe {
		crate::append_utf8(buf, |b| {
			let len = b.len();
			b.set_len(len + count);
			source.read_bytes(&mut b[len..])
				  .map(<[u8]>::len)
		})
	}
}

#[cfg(feature = "alloc")]
#[cfg(test)]
mod read_exact_test {
	use std::assert_matches::assert_matches;
	use proptest::prelude::*;
	use alloc::vec::from_elem;
	use proptest::collection::vec;
	use crate::{BufferAccess, DataSource, Result};

	struct FakeBufSource {
		source: Vec<u8>,
		buffer: Vec<u8>
	}

	#[cfg(feature = "nightly_specialization")]
	impl BufferAccess for FakeBufSource {
		fn buf_capacity(&self) -> usize {
			self.buffer.capacity()
		}

		fn buf(&self) -> &[u8] {
			&self.buffer
		}

		fn fill_buf(&mut self) -> Result<&[u8]> {
			let Self { source, buffer } = self;
			let len = buffer.len();
			buffer.fill(0);
			let source_slice = &mut &source[..];
			let consumed = source_slice.read_bytes(&mut buffer[len..])?.len();
			source.drain(..consumed);
			buffer.truncate(consumed + len);
			Ok(buffer)
		}

		fn clear_buf(&mut self) {
			self.buffer.clear();
		}

		fn consume(&mut self, count: usize) {
			if count == self.buffer.len() {
				self.clear_buf();
			} else {
				self.buffer.drain(..count);
			}
		}
	}

	proptest! {
		#[test]
		fn read_exact_end_of_stream(source in vec(any::<u8>(), 1..=256)) {
			let mut buf = from_elem(0, source.len() + 1);
			assert_matches!(
				super::default_read_exact_bytes(&mut &*source, &mut buf),
				Err(super::Error::End { .. })
			)
		}
	}

	proptest! {
		#[test]
		fn buf_read_exact_end_of_stream(source in vec(any::<u8>(), 1..=256)) {
			let mut buf = from_elem(0, source.len() + 1);
			assert_matches!(
				super::buf_read_exact_bytes(&mut &*source, &mut buf),
				Err(super::Error::End { .. })
			)
		}
	}

	#[cfg(feature = "nightly_specialization")]
	proptest! {
		#[test]
		fn read_exact_insufficient_buffer(source in vec(any::<u8>(), 1..=256)) {
			let source_len = source.len();
			let buffer = Vec::with_capacity(source_len - 1);
			let mut source = FakeBufSource { source, buffer };
			let mut target = from_elem(0, source_len);
			source.read_exact_bytes(&mut target).map(<[u8]>::len).unwrap();
		}
	}

	#[cfg(feature = "nightly_specialization")]
	proptest! {
		#[test]
		fn read_exact_buffered(source in vec(any::<u8>(), 1..=256)) {
			let source_len = source.len();
			let buffer = Vec::with_capacity(source_len + 1);
			let mut source = FakeBufSource { source, buffer };
			let mut target = from_elem(0, source_len);
			source.read_exact_bytes(&mut target).map(<[u8]>::len).unwrap();
		}
	}
}
