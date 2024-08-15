// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

use bytemuck::{bytes_of_mut, Pod};
use num_traits::PrimInt;
#[cfg(feature = "utf8")]
use simdutf8::compat::from_utf8;
use crate::{Error, Result};

mod exact_size;
mod impls;
pub mod markers;

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
			Err(Error::end(count))
		}
	}

	/// Consumes up to `count` bytes in the stream, returning the number of bytes
	/// consumed if successful. At least the available count may be consumed.
	///
	/// # Errors
	///
	/// Returns any IO errors encountered.
	fn skip(&mut self, count: usize) -> Result<usize>;
	/// Reads bytes into a slice, returning the bytes read. This method is greedy;
	/// it consumes as many bytes as it can, until `buf` is filled or no more bytes
	/// are read.
	///
	/// # Errors
	///
	/// Returns any IO errors encountered.
	fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]>;
	/// Reads the exact length of bytes into a slice, returning the bytes read if
	/// successful, or an end-of-stream error if not. Bytes are not consumed if an
	/// end-of-stream error is returned.
	///
	/// # Errors
	///
	/// Returns [`Error::End`] with the slice length if the exact number of bytes
	/// cannot be read. The bytes that were read remain in the buffer, but have
	/// been consumed from the source.
	fn read_exact_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		default_read_exact_bytes(self, buf)
	}
	/// Reads an array with a size of `N` bytes.
	///
	/// # Errors
	///
	/// Returns [`Error::End`] with the array length if [`N`] bytes cannot be read.
	fn read_array<const N: usize>(&mut self) -> Result<[u8; N]>
	where
		Self: Sized
	{
		default_read_array(self)
	}

	/// Reads a [`u8`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `1` byte can be
	/// read.
	fn read_u8(&mut self) -> Result<u8> { self.read_data() }
	/// Reads an [`i8`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `1` byte can be
	/// read.
	fn read_i8(&mut self) -> Result<i8> { self.read_data() }
	/// Reads a big-endian [`u16`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `2` bytes can be
	/// read.
	fn read_u16(&mut self) -> Result<u16> { self.read_int() }
	/// Reads a big-endian [`i16`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `2` bytes can be
	/// read.
	fn read_i16(&mut self) -> Result<i16> { self.read_int() }
	/// Reads a little-endian [`u16`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `2` bytes can be
	/// read.
	fn read_u16_le(&mut self) -> Result<u16> { self.read_int_le() }
	/// Reads a little-endian [`i16`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `2` bytes can be
	/// read.
	fn read_i16_le(&mut self) -> Result<i16> { self.read_int_le() }
	/// Reads a big-endian [`u32`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `4` bytes can be
	/// read.
	fn read_u32(&mut self) -> Result<u32> { self.read_int() }
	/// Reads a big-endian [`i32`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `4` bytes can be
	/// read.
	fn read_i32(&mut self) -> Result<i32> { self.read_int() }
	/// Reads a little-endian [`u32`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `4` bytes can be
	/// read.
	fn read_u32_le(&mut self) -> Result<u32> { self.read_int_le() }
	/// Reads a little-endian [`i32`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `4` bytes can be
	/// read.
	fn read_i32_le(&mut self) -> Result<i32> { self.read_int_le() }
	/// Reads a big-endian [`u64`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `8` bytes can be
	/// read.
	fn read_u64(&mut self) -> Result<u64> { self.read_int() }
	/// Reads a big-endian [`i64`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `8` bytes can be
	/// read.
	fn read_i64(&mut self) -> Result<i64> { self.read_int() }
	/// Reads a little-endian [`u64`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `8` bytes can be
	/// read.
	fn read_u64_le(&mut self) -> Result<u64> { self.read_int_le() }
	/// Reads a little-endian [`i64`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `8` bytes can be
	/// read.
	fn read_i64_le(&mut self) -> Result<i64> { self.read_int_le() }
	/// Reads a big-endian [`u128`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `16` bytes can be
	/// read.
	fn read_u128(&mut self) -> Result<u128> { self.read_int() }
	/// Reads a big-endian [`i128`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `16` bytes can be
	/// read.
	fn read_i128(&mut self) -> Result<i128> { self.read_int() }
	/// Reads a little-endian [`u128`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `16` bytes can be
	/// read.
	fn read_u128_le(&mut self) -> Result<u128> { self.read_int_le() }
	/// Reads a little-endian [`i128`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `16` bytes can be
	/// read.
	fn read_i128_le(&mut self) -> Result<i128> { self.read_int_le() }
	/// Reads a big-endian [`usize`]. To make streams consistent across platforms,
	/// [`usize`] is fixed to the size of [`u64`] regardless of the target platform.
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `8` bytes can be
	/// read.
	fn read_usize(&mut self) -> Result<usize> {
		self.read_u64().map(|i| i as usize)
	}
	/// Reads a big-endian [`isize`]. To make streams consistent across platforms,
	/// [`isize`] is fixed to the size of [`i64`] regardless of the target platform.
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `8` bytes can be
	/// read.
	fn read_isize(&mut self) -> Result<isize> {
		self.read_i64().map(|i| i as isize)
	}
	/// Reads a little-endian [`usize`]. To make streams consistent across platforms,
	/// [`usize`] is fixed to the size of [`u64`] regardless of the target platform.
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `8` bytes can be
	/// read.
	fn read_usize_le(&mut self) -> Result<usize> {
		self.read_u64_le().map(|i| i as usize)
	}
	/// Reads a little-endian [`isize`]. To make streams consistent across platforms,
	/// [`isize`] is fixed to the size of [`i64`] regardless of the target platform.
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `8` bytes can be
	/// read.
	fn read_isize_le(&mut self) -> Result<isize> {
		self.read_i64_le().map(|i| i as isize)
	}

	/// Reads bytes into a slice, returning them as a UTF-8 string if valid.
	///
	/// # Errors
	///
	/// Returns [`Error::Utf8`] if invalid UTF-8 is read. The stream is left in an
	/// undefined state with up to `buf.len()` bytes consumed, including invalid
	/// bytes and any subsequent bytes. `buf` contains at least any valid UTF-8
	/// read before invalid bytes were encountered. The valid UTF-8 length is given
	/// by the error, [`Utf8Error::valid_up_to`]. This slice can be safely converted
	/// to a string with [`from_utf8_unchecked`] or [`Utf8Error::split_valid`]:
	///
	/// ```
	/// # use data_streams::{DataSource, Error};
	/// # let mut source = &[b'h', b'e', b'l', b'l', b'o', 0xFF][..];
	/// # let buffer = &mut [0; 6];
	/// let str: &str = match source.read_utf8(buffer) {
	///     Ok(str) => str,
	///     Err(Error::Utf8(error)) => {
	///         let (valid, invalid) = unsafe {
	///             // Safe because the buffer has been validated up to this point,
	///             // according to the error.
	///             error.split_valid(buffer)
	///         };
	///         // Do something with invalid bytes...
	///         valid
	///     }
	///     Err(error) => return Err(error)
	/// };
	/// # assert_eq!(str, "hello");
	/// # Ok::<_, Error>(())
	/// ```
	///
	/// [`from_utf8_unchecked`]: core::str::from_utf8_unchecked
	///
	/// # Implementation
	///
	/// The default implementation uses a very fast UTF-8 validator ([`simdutf8`]),
	/// so overriding is unlikely to be useful.
	///
	/// [`simdutf8`]: https://crates.io/crates/simdutf8
	#[cfg(feature = "utf8")]
	fn read_utf8<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a str> {
		let bytes = self.read_bytes(buf)?;
		let utf8 = from_utf8(bytes)?;
		Ok(utf8)
	}
}

/// A helper macro which conditionally disables the default body of a method if
/// the specialization feature-gate is not enabled.
#[cfg(feature = "alloc")]
macro_rules! spec_default {
    ($(#[$meta:meta])+fn $name:ident<$lt:lifetime>(&mut $self:ident, $arg:ident: $arg_ty:ty) -> $result:ty $body:block) => {
		$(#[$meta])+
		#[cfg(feature = "unstable_specialization")]
		fn $name<$lt>(&mut $self, $arg: $arg_ty) -> $result $body
		$(#[$meta])+
		#[cfg(not(feature = "unstable_specialization"))]
		fn $name<$lt>(&mut $self, $arg: $arg_ty) -> $result;
	};
}

/// A source stream reading data into vectors.
#[cfg(feature = "alloc")]
pub trait VecSource: DataSource {
	spec_default! {
	/// Reads bytes into `buf` until the presumptive end of the stream, returning
	/// the bytes read. If an error is returned, any bytes read remain in `buf`.
	///
	/// Note that the stream may not necessarily have ended; more bytes may still
	/// be read in subsequent calls. The stream's end is only *presumed* to be
	/// reached. For example, a TCP socket may read no data signaling an end, but
	/// later begin reading again.
	///
	/// # Errors
	///
	/// Returns any IO errors encountered.
	fn read_to_end<'a>(&mut self, buf: &'a mut alloc::vec::Vec<u8>) -> Result<&'a [u8]> {
		impls::read_to_end(self, buf, 0)
	}
	}
	
	spec_default! {
	/// Reads UTF-8 bytes into `buf` until the end of the stream, returning the
	/// string read. If invalid bytes are encountered, an error is returned and
	/// `buf` is unchanged. In this case, the stream is left in a state with an
	/// undefined number of bytes read.
	///
	/// # Errors
	///
	/// Returns [`Error::Utf8`] if invalid UTF-8 is read. The stream is left in a
	/// state with all bytes consumed from it. `buf` contains the read UTF-8 string
	/// up to the invalid bytes.
	#[cfg(feature = "utf8")]
	fn read_utf8_to_end<'a>(&mut self, buf: &'a mut alloc::string::String) -> Result<&'a str> {
		unsafe {
			append_utf8(buf, |buf| impls::read_to_end(self, buf, 0).map(<[u8]>::len))
		}
	}
	}
}

/// Reads generic data from a [source](DataSource).
pub trait GenericDataSource<T: Pod>: DataSource {
	/// Reads a big-endian integer.
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly the type's size in
	/// bytes can be read.
	fn read_int(&mut self) -> Result<T> where T: PrimInt {
		self.read_data().map(T::from_be)
	}

	/// Reads a little-endian integer.
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly the type's size in
	/// bytes can be read.
	fn read_int_le(&mut self) -> Result<T> where T: PrimInt {
		self.read_data().map(T::from_le)
	}

	/// Reads a value of generic type `T` supporting an arbitrary bit pattern. See
	/// [`Pod`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly the type's size in
	/// bytes can be read.
	fn read_data(&mut self) -> Result<T> {
		let mut value = T::zeroed();
		self.read_exact_bytes(bytes_of_mut(&mut value))?;
		Ok(value)
	}
}

impl<S: DataSource + ?Sized, T: Pod> GenericDataSource<T> for S { }

/// Accesses a source's internal buffer.
pub trait BufferAccess: DataSource {
	/// Returns the capacity of the internal buffer.
	fn buffer_capacity(&self) -> usize;
	/// Returns the byte count contained in the internal buffer.
	fn buffer_count(&self) -> usize { self.buffer().len() }
	/// Returns a slice over the filled portion of the internal buffer. This slice
	/// may not contain the whole buffer, for example if it can't be represented as
	/// just one slice.
	fn buffer(&self) -> &[u8];
	/// Fills the internal buffer from the underlying stream, returning its contents
	/// if successful.
	/// 
	/// # Errors
	/// 
	/// Returns any IO errors encountered.
	fn fill_buffer(&mut self) -> Result<&[u8]>;
	/// Clears the internal buffer.
	fn clear_buffer(&mut self) {
		self.drain_buffer(self.buffer_count());
	}
	/// Consumes `count` bytes from the internal buffer. The `count` must be `<=`
	/// the length of the slice returned by either [`buffer`](Self::buffer) or
	/// [`fill_buffer`](Self::fill_buffer)
	/// 
	/// # Panics
	/// 
	/// This method panics if `count` exceeds the buffer length.
	fn drain_buffer(&mut self, count: usize);
	/// Bypasses the internal buffer by returning the underlying source, or `self`
	/// if this behavior is not supported. Note that not fully draining the buffer
	/// before bypassing it will cause data loss.
	fn bypass_buffer(&mut self) -> &mut impl DataSource where Self: Sized {
		self.clear_buffer();
		self
	}
}

#[cfg(feature = "unstable_specialization")]
impl<T: BufferAccess + ?Sized> DataSource for T {
	default fn available(&self) -> usize {
		self.buffer_count()
	}

	default fn request(&mut self, count: usize) -> Result<bool> {
		default_request(self, count)
	}

	default fn skip(&mut self, count: usize) -> Result<usize> {
		Ok(default_skip(self, count))
	}

	default fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		let mut slice = &mut *buf;
		while !slice.is_empty() {
			let mut buf = match self.request(slice.len()) {
				Ok(_) => self.buffer(),
				Err(Error::InsufficientBuffer { .. }) => self.fill_buffer()?,
				Err(error) => return Err(error)
			};
			if buf.is_empty() {
				break
			}

			let count = buf.read_bytes(slice).unwrap().len();
			self.drain_buffer(count);
			slice = &mut slice[count..];
		}

		let unfilled = slice.len();
		let filled = buf.len() - unfilled;
		Ok(&buf[..filled])
	}

	default fn read_exact_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		buf_read_exact_bytes(self, buf)
	}

	#[cfg(feature = "utf8")]
	default fn read_utf8<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a str> {
		let mut slice = &mut *buf;
		let mut valid_len = 0;
		while !slice.is_empty() {
			let mut buf = match self.request(slice.len()) {
				Ok(_) => self.buffer(),
				Err(Error::InsufficientBuffer { .. }) => self.fill_buffer()?,
				Err(error) => return Err(error)
			};
			if buf.is_empty() {
				break
			}

			let count = match buf.read_utf8(slice) {
				Ok(str) => str.len(),
				Err(Error::Utf8(error)) =>
					return Err(error.with_offset(valid_len).into()),
				Err(error) => return Err(error)
			};
			valid_len += count;
			slice = &mut slice[count..];
		}

		// Safety: valid_len bytes have been validated as UTF-8.
		Ok(unsafe { core::str::from_utf8_unchecked(&buf[..valid_len]) })
	}
}

#[cfg(all(feature = "alloc", feature = "unstable_specialization"))]
impl<T: BufferAccess> VecSource for T {
	default fn read_to_end<'a>(&mut self, buf: &'a mut alloc::vec::Vec<u8>) -> Result<&'a [u8]> {
		impls::buf_read_to_end(self, buf)
	}

	#[cfg(feature = "utf8")]
	default fn read_utf8_to_end<'a>(&mut self, buf: &'a mut alloc::string::String) -> Result<&'a str> {
		impls::buf_read_utf8_to_end(self, buf)
	}
}

#[allow(dead_code)]
pub(crate) fn default_request(source: &mut (impl BufferAccess + ?Sized), count: usize) -> Result<bool> {
	if source.available() < count {
		let buf_len = source.buffer_count();
		let spare_capacity = source.buffer_capacity() - buf_len;
		if source.buffer_capacity() > 0 && count < spare_capacity {
			Ok(source.fill_buffer()?.len() >= count)
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

// Todo: after consuming, loop fill_buf and consume.
#[allow(dead_code)]
pub(crate) fn default_skip(source: &mut (impl BufferAccess + ?Sized), mut count: usize) -> usize {
	let avail = source.available();
	count = count.min(avail);
	source.drain_buffer(count);
	// Guard against faulty implementations by verifying that the buffered
	// bytes were removed.
	assert_eq!(source.available(), avail.saturating_sub(count));
	avail
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
	let read_count = source.read_bytes(&mut buf[filled..])?.len();
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

#[cfg(feature = "unstable_specialization")]
fn buf_read_exact_bytes<'a>(source: &mut (impl BufferAccess + ?Sized), buf: &'a mut [u8]) -> Result<&'a [u8]> {
	let len = buf.len();
	match source.require(len) {
		Ok(()) => try_read_exact_contiguous(source, buf),
		Err(Error::InsufficientBuffer { .. }) => {
			// We're doing a large read. Drain the internal buffer, then try reading.
			// Most default implementations of read_bytes optimize for this case by
			// skipping the buffer.

			let mut slice = &mut *buf;
			let mut s_buf = source.buffer();
			while !slice.is_empty() && !s_buf.is_empty() {
				let len = s_buf.read_bytes(slice)?.len();
				slice = &mut slice[len..];
				source.drain_buffer(len);
				s_buf = source.buffer();
			}

			let remaining = slice.len();
			try_read_exact_discontiguous(source, buf, remaining)
		}
		Err(error) => Err(error)
	}
}

#[cfg(all(feature = "alloc", feature = "utf8"))]
#[allow(dead_code)]
pub(crate) fn default_read_utf8<'a>(
	source: &mut (impl DataSource + ?Sized),
	count: usize,
	buf: &'a mut alloc::string::String
) -> Result<&'a str> {
	buf.reserve(count);
	unsafe {
		append_utf8(buf, |b| {
			let len = b.len();
			b.set_len(len + count);
			source.read_bytes(&mut b[len..])
				  .map(<[u8]>::len)
		})
	}
}

#[cfg(all(feature = "alloc", feature = "utf8"))]
#[allow(dead_code)]
pub(crate) unsafe fn append_utf8<R>(buf: &mut alloc::string::String, read: R) -> Result<&str>
where
	R: FnOnce(&mut alloc::vec::Vec<u8>) -> Result<usize> {
	use simdutf8::compat::from_utf8;

	// A drop guard which ensures the string is truncated to valid UTF-8 when out
	// of scope. Starts by truncating to its original length, only allowing the
	// string to grow after the new bytes are checked to be valid UTF-8.
	struct Guard<'a> {
		len: usize,
		buf: &'a mut alloc::vec::Vec<u8>
	}

	impl Drop for Guard<'_> {
		fn drop(&mut self) {
			unsafe {
				self.buf.set_len(self.len);
			}
		}
	}

	let start;
	{
		let mut guard = Guard { len: buf.len(), buf: buf.as_mut_vec() };
		let count = read(guard.buf)?;
		from_utf8(&guard.buf[guard.len..][..count])?;
		start = guard.len;
		guard.len += count;
	}
	Ok(&buf[start..])
}

#[cfg(all(
	test,
	feature = "std",
	feature = "alloc",
	feature = "unstable_specialization"
))]
mod read_exact_test {
	use std::assert_matches::assert_matches;
	use proptest::prelude::*;
	use alloc::vec::from_elem;
	use std::iter::repeat;
	use proptest::collection::vec;
	use crate::{BufferAccess, DataSource, Result};
	
	struct FakeBufSource {
		source: Vec<u8>,
		buffer: Vec<u8>
	}

	impl BufferAccess for FakeBufSource {
		fn buffer_capacity(&self) -> usize {
			self.buffer.capacity()
		}

		fn buffer(&self) -> &[u8] {
			&self.buffer
		}

		fn fill_buffer(&mut self) -> Result<&[u8]> {
			let Self { source, buffer } = self;
			let len = buffer.len();
			buffer.extend(repeat(0).take(buffer.capacity() - len));
			let source_slice = &mut &source[..];
			let consumed = source_slice.read_bytes(&mut buffer[len..])?.len();
			source.drain_buffer(consumed);
			buffer.truncate(consumed + len);
			Ok(buffer)
		}

		fn clear_buffer(&mut self) {
			self.buffer.clear();
		}

		fn drain_buffer(&mut self, count: usize) {
			self.buffer.drain_buffer(count);
		}
	}

	proptest! {
		#[test]
		fn read_exact_end_of_stream(source in vec(any::<u8>(), 1..=256)) {
			let mut buf = from_elem(0, source.len() + 1);
			assert_matches!(
				super::default_read_exact_bytes(&mut &*source, &mut buf),
				Err(super::Error::End { .. })
			);
		}
	}

	proptest! {
		#[test]
		fn buf_read_exact_end_of_stream(source in vec(any::<u8>(), 1..=256)) {
			let mut buf = from_elem(0, source.len() + 1);
			assert_matches!(
				super::buf_read_exact_bytes(&mut &*source, &mut buf),
				Err(super::Error::End { .. })
			);
		}
	}

	proptest! {
		#[test]
		fn read_exact_insufficient_buffer(source in vec(any::<u8>(), 2..=256)) {
			let source_len = source.len();
			let buffer = Vec::with_capacity(source_len - 1);
			let mut source = FakeBufSource { source, buffer };
			let mut target = from_elem(0, source_len);
			source.read_exact_bytes(&mut target).map(<[u8]>::len).unwrap();
		}
	}

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
