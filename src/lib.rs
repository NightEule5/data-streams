// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate core;
#[cfg(feature = "alloc")]
extern crate alloc;

mod slice;
mod std_io;

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};
use core::fmt;
#[cfg(feature = "std")]
use std::io;
use bytemuck::{bytes_of, bytes_of_mut, Pod};
use num_traits::PrimInt;
#[cfg(feature = "alloc")]
use simdutf8::compat::Utf8Error;

#[derive(Debug)]
pub enum Error {
	#[cfg(feature = "std")]
	Io(io::Error),
	#[cfg(feature = "alloc")]
	Utf8(Utf8Error),
	End {
		required_count: usize
	},
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			#[cfg(feature = "std")]
			Self::Io(error) => Some(error),
			#[cfg(feature = "alloc")]
			Self::Utf8(error) => Some(error),
			Self::End { .. } => None,
		}
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			#[cfg(feature = "std")]
			Self::Io(error) => fmt::Display::fmt(error, f),
			#[cfg(feature = "alloc")]
			Self::Utf8(error) => fmt::Display::fmt(error, f),
			Self::End { required_count } => write!(f, "premature end-of-stream when reading {required_count} bytes"),
		}
	}
}

#[cfg(feature = "std")]
impl From<io::Error> for Error {
	fn from(value: io::Error) -> Self {
		Self::Io(value)
	}
}

#[cfg(feature = "alloc")]
impl From<Utf8Error> for Error {
	fn from(value: Utf8Error) -> Self {
		Self::Utf8(value)
	}
}

pub type Result<T = (), E = Error> = core::result::Result<T, E>;

/// A source stream of data.
pub trait DataSource {
	/// Returns the number of bytes available for reading.
	fn available(&self) -> usize;
	/// Reads at most `count` bytes into an internal buffer, returning whether
	/// enough bytes are available. To return an end-of-stream error, use [`require`]
	/// instead.
	///
	/// Note that a request returning `false` doesn't necessarily mean the stream
	/// has ended. More bytes may be read after.
	///
	/// [`require`]: Self::require
	fn request(&mut self, count: usize) -> Result<bool>;
	/// Reads at least `count` bytes into an internal buffer, returning the available
	/// count if successful, or an end-of-stream error if not. For a softer version
	/// that returns whether enough bytes are available, use [`request`].
	///
	/// [`request`]: Self::request
	fn require(&mut self, count: usize) -> Result {
		if self.request(count)? {
			Ok(())
		} else {
			Err(Error::End { required_count: count })
		}
	}

	/// Reads bytes into a slice, returning the bytes read.
	fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]>;
	/// Reads the exact length of bytes into a slice, returning the bytes read if
	/// successful, or an end-of-stream error if not. Bytes are not consumed if an
	/// end-of-stream error is returned.
	fn read_exact_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		let len = buf.len();
		self.require(len)?;
		let bytes = self.read_bytes(buf)?;
		assert_eq!(bytes.len(), len);
		Ok(bytes)
	}
	/// Reads an array with a size of `N` bytes.
	fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
		let mut array = [0; N];
		self.read_exact_bytes(&mut array)?;
		Ok(array)
	}

	/// Reads a [`u8`].
	fn read_u8(&mut self) -> Result<u8> { self.read_int() }
	/// Reads an [`i8`].
	fn read_i8(&mut self) -> Result<i8> { self.read_int() }
	/// Reads a big-endian [`u16`].
	fn read_u16(&mut self) -> Result<u16> { self.read_int() }
	/// Reads a big-endian [`i16`].
	fn read_i16(&mut self) -> Result<i16> { self.read_int() }
	/// Reads a little-endian [`u16`].
	fn read_u16_le(&mut self) -> Result<u16> { self.read_int_le() }
	/// Reads a little-endian [`i16`].
	fn read_i16_le(&mut self) -> Result<i16> { self.read_int_le() }
	/// Reads a big-endian [`u32`].
	fn read_u32(&mut self) -> Result<u32> { self.read_int() }
	/// Reads a big-endian [`i32`].
	fn read_i32(&mut self) -> Result<i32> { self.read_int() }
	/// Reads a little-endian [`u32`].
	fn read_u32_le(&mut self) -> Result<u32> { self.read_int_le() }
	/// Reads a little-endian [`i32`].
	fn read_i32_le(&mut self) -> Result<i32> { self.read_int_le() }
	/// Reads a big-endian [`u64`].
	fn read_u64(&mut self) -> Result<u64> { self.read_int() }
	/// Reads a big-endian [`i64`].
	fn read_i64(&mut self) -> Result<i64> { self.read_int() }
	/// Reads a little-endian [`u64`].
	fn read_u64_le(&mut self) -> Result<u64> { self.read_int_le() }
	/// Reads a little-endian [`i64`].
	fn read_i64_le(&mut self) -> Result<i64> { self.read_int_le() }
	/// Reads a big-endian [`u128`].
	fn read_u128(&mut self) -> Result<u128> { self.read_int() }
	/// Reads a big-endian [`i128`].
	fn read_i128(&mut self) -> Result<i128> { self.read_int() }
	/// Reads a little-endian [`u128`].
	fn read_u128_le(&mut self) -> Result<u128> { self.read_int_le() }
	/// Reads a little-endian [`i128`].
	fn read_i128_le(&mut self) -> Result<i128> { self.read_int_le() }
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
	fn read_int<T: PrimInt + Pod>(&mut self) -> Result<T> {
		self.read_data::<T>().map(T::from_be)
	}
	/// Reads a little-endian integer.
	fn read_int_le<T: PrimInt + Pod>(&mut self) -> Result<T> {
		self.read_data::<T>().map(T::from_le)
	}

	/// Reads a value of generic type `T` supporting an arbitrary bit pattern. See
	/// [`Pod`].
	fn read_data<T: Pod>(&mut self) -> Result<T> {
		let mut value = T::zeroed();
		self.read_exact_bytes(bytes_of_mut(&mut value))?;
		Ok(value)
	}

	/// Reads up to `count` bytes of UTF-8 into `buf`, returning the string read.
	/// If invalid bytes are encountered, an error is returned and `buf` is unchanged.
	/// In this case, the stream is left in a state with up to `count` bytes consumed
	/// from it, including the invalid bytes and any subsequent bytes.
	#[cfg(feature = "alloc")]
	fn read_utf8<'a>(&mut self, count: usize, buf: &'a mut String) -> Result<&'a str> {
		buf.reserve(count);
		unsafe {
			append_utf8(buf, |b| {
				let len = b.len();
				b.set_len(len + count);
				self.read_bytes(&mut b[len..])
					.map(<[u8]>::len)
			})
		}
	}

	/// Reads UTF-8 bytes into `buf` until the end of the stream, returning the
	/// string read. If invalid bytes are encountered, an error is returned and
	/// `buf` is unchanged. In this case, the stream is left in a state with an
	/// undefined number of bytes read.
	#[cfg(feature = "alloc")]
	fn read_utf8_to_end<'a>(&mut self, buf: &'a mut String) -> Result<&'a str>;
}

pub trait DataSink {
	/// Writes all bytes from `buf`. Equivalent to [`Write::write_all`].
	/// 
	/// [`Write::write_all`]: io::Write::write_all
	fn write_bytes(&mut self, buf: &[u8]) -> Result;

	/// Writes a [`u8`].
	fn write_u8(&mut self, value: u8) -> Result { self.write_int(value) }
	/// Writes an [`i8`].
	fn write_i8(&mut self, value: i8) -> Result { self.write_int(value) }
	/// Writes a big-endian [`u16`].
	fn write_u16(&mut self, value: u16) -> Result { self.write_int(value) }
	/// Writes a big-endian [`i16`].
	fn write_i16(&mut self, value: i16) -> Result { self.write_int(value) }
	/// Writes a little-endian [`u16`].
	fn write_u16_le(&mut self, value: u16) -> Result { self.write_int_le(value) }
	/// Writes a little-endian [`i16`].
	fn write_i16_le(&mut self, value: i16) -> Result { self.write_int_le(value) }
	/// Writes a big-endian [`u32`].
	fn write_u32(&mut self, value: u32) -> Result { self.write_int(value) }
	/// Writes a big-endian [`i32`].
	fn write_i32(&mut self, value: i32) -> Result { self.write_int(value) }
	/// Writes a little-endian [`u32`].
	fn write_u32_le(&mut self, value: u32) -> Result { self.write_int_le(value) }
	/// Writes a little-endian [`i32`].
	fn write_i32_le(&mut self, value: i32) -> Result { self.write_int_le(value) }
	/// Writes a big-endian [`u64`].
	fn write_u64(&mut self, value: u64) -> Result { self.write_int(value) }
	/// Writes a big-endian [`i64`].
	fn write_i64(&mut self, value: i64) -> Result { self.write_int(value) }
	/// Writes a little-endian [`u64`].
	fn write_u64_le(&mut self, value: u64) -> Result { self.write_int_le(value) }
	/// Writes a little-endian [`i64`].
	fn write_i64_le(&mut self, value: i64) -> Result { self.write_int_le(value) }
	/// Writes a big-endian [`u128`].
	fn write_u128(&mut self, value: u128) -> Result { self.write_int(value) }
	/// Writes a big-endian [`i128`].
	fn write_i128(&mut self, value: i128) -> Result { self.write_int(value) }
	/// Writes a little-endian [`u128`].
	fn write_u128_le(&mut self, value: u128) -> Result { self.write_int_le(value) }
	/// Writes a little-endian [`i128`].
	fn write_i128_le(&mut self, value: i128) -> Result { self.write_int_le(value) }
	/// Writes a big-endian [`usize`]. To make streams consistent across platforms,
	/// [`usize`] is fixed to the size of [`u64`] regardless of the target platform.
	fn write_usize(&mut self, value: usize) -> Result {
		self.write_u64(value as u64)
	}
	/// Writes a big-endian [`isize`]. To make streams consistent across platforms,
	/// [`isize`] is fixed to the size of [`i64`] regardless of the target platform.
	fn write_isize(&mut self, value: isize) -> Result {
		self.write_i64(value as i64)
	}
	/// Writes a little-endian [`usize`]. To make streams consistent across platforms,
	/// [`usize`] is fixed to the size of [`u64`] regardless of the target platform.
	fn write_usize_le(&mut self, value: usize) -> Result {
		self.write_u64_le(value as u64)
	}
	/// Writes a little-endian [`isize`]. To make streams consistent across platforms,
	/// [`isize`] is fixed to the size of [`i64`] regardless of the target platform.
	fn write_isize_le(&mut self, value: isize) -> Result {
		self.write_i64_le(value as i64)
	}

	/// Writes a big-endian integer.
	fn write_int<T: PrimInt + Pod>(&mut self, value: T) -> Result {
		self.write_data(value.to_be())
	}
	/// Writes a little-endian integer.
	fn write_int_le<T: PrimInt + Pod>(&mut self, value: T) -> Result {
		self.write_data(value.to_le())
	}
	/// Writes a value of an arbitrary bit pattern. See [`Pod`].
	fn write_data<T: Pod>(&mut self, value: T) -> Result {
		self.write_bytes(bytes_of(&value))
	}
	/// Writes a UTF-8 string.
	fn write_utf8(&mut self, value: &str) -> Result {
		self.write_bytes(value.as_bytes())
	}
}

#[cfg(feature = "alloc")]
unsafe fn append_utf8<R>(buf: &mut String, read: R) -> Result<&str>
where
	R: FnOnce(&mut Vec<u8>) -> Result<usize> {
	use simdutf8::compat::from_utf8;

	// A drop guard which ensures the string is truncated to valid UTF-8 when out
	// of scope. Starts by truncating to its original length, only allowing the
	// string to grow after the new bytes are checked to be valid UTF-8.
	struct Guard<'a> {
		len: usize,
		buf: &'a mut Vec<u8>
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
