// Copyright 2025 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

use bytemuck::{bytes_of_mut, cast_slice_mut, Pod};
#[cfg(feature = "unstable_ascii_char")]
use core::ascii;
use bytemuck::cast_slice;
use num_traits::PrimInt;
#[cfg(feature = "utf8")]
use simdutf8::compat::from_utf8;
use crate::{Error, Result};
#[cfg(feature = "utf8")]
use crate::utf8::utf8_char_width;

mod exact_size;
mod impls;
pub mod markers;

/// A source stream of data.
pub trait DataSource {
	/// Returns the number of bytes available for reading. This does not necessarily
	/// mean more data isn't available, just that *at least* this count is may be
	/// read.
	/// 
	/// # Example
	/// 
	/// ```
	/// use data_streams::DataSource;
	/// 
	/// let buf: &[u8] = b"Hello!";
	/// assert_eq!(buf.available(), 6);
	/// ```
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
	/// 
	/// # Example
	/// 
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = b"Hello!";
	/// assert_eq!(buf.request(3)?, true);
	/// assert_eq!(buf.request(50)?, false);
	/// # Ok::<_, Error>(())
	/// ```
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
	/// 
	/// # Example
	/// 
	/// ```
	/// use data_streams::{DataSource, Error};
	/// 
	/// let mut buf: &[u8] = b"Hello!";
	/// assert!(buf.require(3).is_ok());
	/// assert!(matches!(buf.require(50), Err(Error::End { .. })));
	/// ```
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
	/// 
	/// # Example
	/// 
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	/// 
	/// let mut buf: &[u8] = b"Hello!";
	/// assert_eq!(buf.skip(3)?, 3);
	/// assert_eq!(buf.skip(8)?, 3);
	/// # Ok::<_, Error>(())
	/// ```
	fn skip(&mut self, count: usize) -> Result<usize>;
	/// Reads bytes into a slice, returning the bytes read. This method is greedy;
	/// it consumes as many bytes as it can, until `buf` is filled or no more bytes
	/// are read.
	///
	/// # Errors
	///
	/// Returns any IO errors encountered.
	/// 
	/// # Example
	/// 
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	/// 
	/// let mut input: &[u8] = b"Hello!";
	/// let buf: &mut [u8] = &mut [0; 5];
	/// assert_eq!(input.read_bytes(&mut buf[..3])?, b"Hel");
	/// assert_eq!(input.read_bytes(buf)?, b"lo!");
	/// # Ok::<_, Error>(())
	/// ```
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
	/// 
	/// # Example
	/// 
	/// ```
	/// use data_streams::{DataSource, Error};
	/// 
	/// let mut input: &[u8] = b"Hello!";
	/// let buf: &mut [u8] = &mut [0; 5];
	/// assert_eq!(input.read_exact_bytes(&mut buf[..3])?, b"Hel");
	/// assert!(matches!(input.read_exact_bytes(buf), Err(Error::End { .. })));
	/// # Ok::<_, Error>(())
	/// ```
	fn read_exact_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		default_read_exact_bytes(self, buf)
	}
	/// Reads bytes into a slice in multiples of `alignment`, returning the bytes
	/// read. This method is greedy; it consumes as many bytes as it can, until
	/// `buf` is filled or less than `alignment` bytes could be read.
	/// 
	/// If the alignment is zero, the returned slice is empty.
	/// 
	/// # Errors
	/// 
	/// Returns any IO errors encountered.
	/// 
	/// # Example
	/// 
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	/// 
	/// let mut input: &[u8] = b"Hello?!";
	/// let buf: &mut [u8] = &mut [0; 10];
	/// assert_eq!(input.read_aligned_bytes(buf, 2)?, b"Hello?");
	/// assert_eq!(input.read_aligned_bytes(buf, 2)?, b"");
	/// # Ok::<_, Error>(())
	/// ```
	fn read_aligned_bytes<'a>(&mut self, buf: &'a mut [u8], alignment: usize) -> Result<&'a [u8]> {
		default_read_aligned_bytes(self, buf, alignment)
	}
	/// Reads an array with a size of `N` bytes.
	///
	/// # Errors
	///
	/// Returns [`Error::End`] with the array length if [`N`] bytes cannot be read.
	/// 
	/// # Example
	/// 
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	/// 
	/// let mut input: &[u8] = b"Hello!";
	/// assert_eq!(input.read_array::<3>()?, *b"Hel");
	/// # Ok::<_, Error>(())
	/// ```
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
	/// 
	/// # Example
	/// 
	/// ```
	/// use data_streams::DataSource;
	/// 
	/// let mut buf: &[u8] = &[2, 3, 5, 7, 11];
	/// 
	/// let mut sum = 0;
	/// while let Ok(byte) = buf.read_u8() {
 	///     sum += byte;
	/// }
	/// assert_eq!(sum, 28);
	/// ```
	fn read_u8(&mut self) -> Result<u8> { self.read_data() }
	/// Reads an [`i8`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `1` byte can be
	/// read.
	/// 
	/// ```
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[2, (-3i8) as u8, 5, (-7i8) as u8, 11];
	/// 
	/// let mut sum = 0;
	/// while let Ok(byte) = buf.read_i8() {
	///     sum += byte;
	/// }
	/// assert_eq!(sum, 8);
	/// ```
	fn read_i8(&mut self) -> Result<i8> { self.read_data() }
	/// Reads a big-endian [`u16`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `2` bytes can be
	/// read.
	/// 
	/// # Example
	/// 
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	/// 
	/// let mut buf: &[u8] = &[0x12, 0x34, 0x56, 0x78];
	/// assert_eq!(buf.read_u16()?, 0x1234);
	/// assert_eq!(buf.read_u16()?, 0x5678);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_u16(&mut self) -> Result<u16> { self.read_int() }
	/// Reads a big-endian [`i16`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `2` bytes can be
	/// read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[0x12, 0x34, 0x56, 0x78];
	/// assert_eq!(buf.read_i16()?, 0x1234);
	/// assert_eq!(buf.read_i16()?, 0x5678);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_i16(&mut self) -> Result<i16> { self.read_int() }
	/// Reads a little-endian [`u16`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `2` bytes can be
	/// read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[0x12, 0x34, 0x56, 0x78];
	/// assert_eq!(buf.read_u16_le()?, 0x3412);
	/// assert_eq!(buf.read_u16_le()?, 0x7856);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_u16_le(&mut self) -> Result<u16> { self.read_int_le() }
	/// Reads a little-endian [`i16`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `2` bytes can be
	/// read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[0x12, 0x34, 0x56, 0x78];
	/// assert_eq!(buf.read_i16_le()?, 0x3412);
	/// assert_eq!(buf.read_i16_le()?, 0x7856);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_i16_le(&mut self) -> Result<i16> { self.read_int_le() }
	/// Reads a big-endian [`u32`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `4` bytes can be
	/// read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[0x12, 0x34, 0x56, 0x78];
	/// assert_eq!(buf.read_u32()?, 0x12345678);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_u32(&mut self) -> Result<u32> { self.read_int() }
	/// Reads a big-endian [`i32`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `4` bytes can be
	/// read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[0x12, 0x34, 0x56, 0x78];
	/// assert_eq!(buf.read_i32()?, 0x12345678);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_i32(&mut self) -> Result<i32> { self.read_int() }
	/// Reads a little-endian [`u32`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `4` bytes can be
	/// read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[0x12, 0x34, 0x56, 0x78];
	/// assert_eq!(buf.read_u32_le()?, 0x78563412);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_u32_le(&mut self) -> Result<u32> { self.read_int_le() }
	/// Reads a little-endian [`i32`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `4` bytes can be
	/// read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[0x12, 0x34, 0x56, 0x78];
	/// assert_eq!(buf.read_i32_le()?, 0x78563412);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_i32_le(&mut self) -> Result<i32> { self.read_int_le() }
	/// Reads a big-endian [`u64`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `8` bytes can be
	/// read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[
	///     0x12, 0x34, 0x56, 0x78,
	///     0x9A, 0xBC, 0xDE, 0xF0
	/// ];
	/// assert_eq!(buf.read_u64()?, 0x1234_5678_9ABC_DEF0);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_u64(&mut self) -> Result<u64> { self.read_int() }
	/// Reads a big-endian [`i64`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `8` bytes can be
	/// read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[
	///     0x12, 0x34, 0x56, 0x78,
	///     0x9A, 0xBC, 0xDE, 0xF0
	/// ];
	/// assert_eq!(buf.read_i64()?, 0x1234_5678_9ABC_DEF0);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_i64(&mut self) -> Result<i64> { self.read_int() }
	/// Reads a little-endian [`u64`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `8` bytes can be
	/// read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[
	///     0x12, 0x34, 0x56, 0x78,
	///     0x9A, 0xBC, 0xDE, 0xF0
	/// ];
	/// assert_eq!(buf.read_u64_le()?, 0xF0DE_BC9A_7856_3412);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_u64_le(&mut self) -> Result<u64> { self.read_int_le() }
	/// Reads a little-endian [`i64`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `8` bytes can be
	/// read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[
	///     0x12, 0x34, 0x56, 0x78,
	///     0x9A, 0xBC, 0xDE, 0xF0
	/// ];
	/// assert_eq!(buf.read_i64_le()?, 0xF0DE_BC9A_7856_3412u64 as i64);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_i64_le(&mut self) -> Result<i64> { self.read_int_le() }
	/// Reads a big-endian [`u128`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `16` bytes can be
	/// read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[
	///     0x12, 0x34, 0x56, 0x78,
	///     0x9A, 0xBC, 0xDE, 0xF0,
	///     0x0F, 0xED, 0xCB, 0xA9,
	///     0x87, 0x65, 0x43, 0x21
	/// ];
	/// assert_eq!(buf.read_u128()?, 0x1234_5678_9ABC_DEF0_0FED_CBA9_8765_4321);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_u128(&mut self) -> Result<u128> { self.read_int() }
	/// Reads a big-endian [`i128`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `16` bytes can be
	/// read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[
	///     0x12, 0x34, 0x56, 0x78,
	///     0x9A, 0xBC, 0xDE, 0xF0,
	///     0x0F, 0xED, 0xCB, 0xA9,
	///     0x87, 0x65, 0x43, 0x21
	/// ];
	/// assert_eq!(buf.read_i128()?, 0x1234_5678_9ABC_DEF0_0FED_CBA9_8765_4321);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_i128(&mut self) -> Result<i128> { self.read_int() }
	/// Reads a little-endian [`u128`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `16` bytes can be
	/// read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[
	///     0x12, 0x34, 0x56, 0x78,
	///     0x9A, 0xBC, 0xDE, 0xF0,
	///     0x0F, 0xED, 0xCB, 0xA9,
	///     0x87, 0x65, 0x43, 0x21
	/// ];
	/// assert_eq!(buf.read_u128_le()?, 0x2143_6587_A9CB_ED0F_F0DE_BC9A_7856_3412);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_u128_le(&mut self) -> Result<u128> { self.read_int_le() }
	/// Reads a little-endian [`i128`].
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `16` bytes can be
	/// read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[
	///     0x12, 0x34, 0x56, 0x78,
	///     0x9A, 0xBC, 0xDE, 0xF0,
	///     0x0F, 0xED, 0xCB, 0xA9,
	///     0x87, 0x65, 0x43, 0x21
	/// ];
	/// assert_eq!(buf.read_i128_le()?, 0x2143_6587_A9CB_ED0F_F0DE_BC9A_7856_3412);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_i128_le(&mut self) -> Result<i128> { self.read_int_le() }
	/// Reads a big-endian [`usize`]. To make streams consistent across platforms,
	/// [`usize`] is fixed to the size of [`u64`] regardless of the target platform.
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly `8` bytes can be
	/// read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[
	///     0x12, 0x34, 0x56, 0x78,
	///     0x9A, 0xBC, 0xDE, 0xF0
	/// ];
	/// assert_eq!(buf.read_usize()?, 0x1234_5678_9ABC_DEF0);
	/// # Ok::<_, Error>(())
	/// ```
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
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[
	///     0x12, 0x34, 0x56, 0x78,
	///     0x9A, 0xBC, 0xDE, 0xF0
	/// ];
	/// assert_eq!(buf.read_isize()?, 0x1234_5678_9ABC_DEF0);
	/// # Ok::<_, Error>(())
	/// ```
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
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[
	///     0x12, 0x34, 0x56, 0x78,
	///     0x9A, 0xBC, 0xDE, 0xF0
	/// ];
	/// assert_eq!(buf.read_usize_le()?, 0xF0DE_BC9A_7856_3412);
	/// # Ok::<_, Error>(())
	/// ```
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
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut buf: &[u8] = &[
	///     0x12, 0x34, 0x56, 0x78,
	///     0x9A, 0xBC, 0xDE, 0xF0
	/// ];
	/// assert_eq!(buf.read_isize_le()?, 0xF0DE_BC9A_7856_3412usize as isize);
	/// # Ok::<_, Error>(())
	/// ```
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
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut input: &[u8] = "Hello! 👋".as_bytes();
	/// let buf: &mut [u8] = &mut [0; 11];
	///
	/// assert_eq!(input.read_utf8(buf)?, "Hello! 👋");
	/// # Ok::<_, Error>(())
	/// ```
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
	/// Reads a single UTF-8 codepoint, returning a [`char`] if valid.
	///
	/// # Errors
	///
	/// Returns [`Error::Utf8`] if invalid UTF-8 is read. The stream is left with
	/// one to four bytes consumed, depending on the UTF-8 character width encoded
	/// in the first byte. `buf` contains any consumed bytes.
	///
	/// Returns [`Error::End`] if the end-of-stream is reached before the full
	/// character width is read. `buf` is empty or contains exactly one byte.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut input: &[u8] = "🍉".as_bytes();
	/// assert_eq!(input.read_utf8_codepoint(&mut [0; 4])?, '🍉');
	/// # Ok::<_, Error>(())
	/// ```
	#[cfg(feature = "utf8")]
	fn read_utf8_codepoint(&mut self, buf: &mut [u8; 4]) -> Result<char> {
		let Ok(char) = default_read_utf8_codepoint(self, buf)?.parse() else {
			// Safety: this function promises to produce a UTF-8 string with exactly one character.
			unreachable!()
		};
		Ok(char)
	}
	/// Reads bytes into a slice, returning them as an ASCII slice if valid.
	///
	/// # Errors
	///
	/// Returns [`Error::Ascii`] if a non-ASCII byte is found. The stream is left
	/// in an undefined state with up to `buf.len()` bytes consumed, including the
	/// invalid byte and any subsequent bytes. `buf` contains all consumed bytes.
	/// The valid ASCII length is given by the error, [`AsciiError::valid_up_to`].
	/// The number of bytes consumed after the invalid byte is given by
	/// [`AsciiError::unchecked_count`]. These slices can be safely split with
	/// [`AsciiError::split_valid`]:
	///
	/// ```
	/// #![feature(ascii_char)]
	///
	/// # use data_streams::{DataSource, Error};
	/// # use core::ascii;
	/// # let mut source = &[b'h', b'e', b'l', b'l', b'o', 0xFF][..];
	/// # let buffer = &mut [0; 6];
	/// let str: &[ascii::Char] = match source.read_ascii(buffer) {
	///     Ok(str) => str,
	///     Err(Error::Ascii(error)) => {
	///         let (valid, invalid) = error.split_valid(buffer);
	///         // Do something with invalid bytes...
	///         valid
	///     }
	///     Err(error) => return Err(error)
	/// };
	/// # assert_eq!(str.as_str(), "hello");
	/// # Ok::<_, Error>(())
	/// ```
	///
	/// # Example
	///
	/// ```
	/// #![feature(ascii_char)]
	///
	/// # use data_streams::Error;
	/// use data_streams::DataSource;
	///
	/// let mut input: &[u8] = b"Hello!";
	/// let buf: &mut [u8] = &mut [0; 6];
	/// 
	/// assert_eq!(input.read_ascii(buf)?.as_str(), "Hello!");
	/// # Ok::<_, Error>(())
	/// ```
	#[cfg(feature = "unstable_ascii_char")]
	fn read_ascii<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [ascii::Char]> {
		default_read_ascii(self, buf)
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
	/// 
	/// # Example
	/// 
	/// ```
	/// # use data_streams::Error;
	/// # #[cfg(feature = "unstable_specialization")]
	/// # {
	/// use data_streams::VecSource;
	///
	/// let mut input: &[u8] = b"Hello!";
	/// let mut buf = Vec::new();
	/// assert_eq!(input.read_to_end(&mut buf)?, b"Hello!");
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
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
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::VecSource;
	///
	/// let mut input: &[u8] = b"Hello!";
	/// let mut buf = String::new();
	/// assert_eq!(input.read_utf8_to_end(&mut buf)?, "Hello!");
	/// # Ok::<_, Error>(())
	/// ```
	#[cfg(feature = "utf8")]
	fn read_utf8_to_end<'a>(&mut self, buf: &'a mut alloc::string::String) -> Result<&'a str> {
		// Safety: this function only modifies the string's bytes if the new bytes are found to be
		//  valid UTF-8.
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
	///
	/// # Example
	/// 
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::GenericDataSource;
	/// 
	/// let mut buf: &[u8] = &[0x12, 0x34, 0x56, 0x78];
	/// let int: u32 = buf.read_int()?;
	/// assert_eq!(int, 0x12345678);
	/// # Ok::<_, Error>(())
	/// ```
	fn read_int(&mut self) -> Result<T> where T: PrimInt {
		self.read_data().map(T::from_be)
	}

	/// Reads a little-endian integer.
	///
	/// # Errors
	///
	/// Returns [`Error::End`] if the stream ends before exactly the type's size in
	/// bytes can be read.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// use data_streams::GenericDataSource;
	///
	/// let mut buf: &[u8] = &[0x12, 0x34, 0x56, 0x78];
	/// let int: u32 = buf.read_int_le()?;
	/// assert_eq!(int, 0x78563412);
	/// # Ok::<_, Error>(())
	/// ```
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
	///
	/// # Example
	/// 
	/// ```
	/// # use data_streams::Error;
	/// # #[cfg(target_endian = "little")]
	/// # {
	/// use data_streams::GenericDataSource;
	///
	/// let mut buf: &[u8] = &[0x12, 0x34, 0x56, 0x78];
	/// let int: u32 = buf.read_data()?;
	/// assert_eq!(int, 0x78563412);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn read_data(&mut self) -> Result<T> {
		let mut value = T::zeroed();
		self.read_exact_bytes(bytes_of_mut(&mut value))?;
		Ok(value)
	}
	
	/// Reads multiple values of generic type `T` supporting an arbitrary bit pattern,
	/// returning the read values.
	/// 
	/// # Errors
	/// 
	/// Returns any IO errors encountered.
	/// 
	/// # Panics
	/// 
	/// Panics if the [`DataSource::read_aligned_bytes`] implementation returns an unaligned slice.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # #[cfg(target_endian = "little")]
	/// # {
	/// use data_streams::GenericDataSource;
	///
	/// let mut input: &[u8] = &[0x12, 0x34, 0x56, 0x78, 0xFF];
	/// let buf: &mut [u16] = &mut [0; 3];
	/// assert_eq!(input.read_data_slice(buf)?, [0x3412, 0x7856]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn read_data_slice<'a>(&mut self, buf: &'a mut [T]) -> Result<&'a [T]> {
		let bytes = self.read_aligned_bytes(cast_slice_mut(buf), size_of::<T>())?;
		assert_eq!(bytes.len() % size_of::<T>(), 0, "unaligned read implementation");
		Ok(cast_slice(bytes))
	}
}

impl<S: DataSource + ?Sized, T: Pod> GenericDataSource<T> for S { }

/// Accesses a source's internal buffer.
pub trait BufferAccess: DataSource {
	/// Returns the capacity of the internal buffer.
	/// 
	/// # Example
	/// 
	/// ```
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// use data_streams::BufferAccess;
	///
	/// let buf = Vec::<u8>::with_capacity(16);
	/// assert_eq!(buf.buffer_capacity(), 16);
	/// # }
	/// ```
	fn buffer_capacity(&self) -> usize;
	/// Returns the byte count contained in the internal buffer.
	/// 
	/// # Example
	/// 
	/// ```
	/// use data_streams::BufferAccess;
	/// 
	/// let buf: &[u8] = &[0; 16];
	/// assert_eq!(buf.buffer_count(), 16);
	/// ```
	fn buffer_count(&self) -> usize { self.buffer().len() }
	/// Returns a slice over the filled portion of the internal buffer. This slice
	/// may not contain the whole buffer, for example if it can't be represented as
	/// just one slice.
	/// 
	/// # Example
	/// 
	/// ```
	/// use data_streams::BufferAccess;
	/// 
	/// let buf: &[u8] = b"Hello!";
	/// assert_eq!(buf.buffer(), b"Hello!");
	/// ```
	fn buffer(&self) -> &[u8];
	/// Fills the internal buffer from the underlying stream, returning its contents
	/// if successful.
	/// 
	/// # Errors
	/// 
	/// Returns any IO errors encountered.
	///
	/// # Example
	///
	/// ```no_run
	/// # use data_streams::Error;
	/// # #[cfg(feature = "std")]
	/// # {
	/// use std::{fs::File, io::BufReader};
	/// use data_streams::BufferAccess;
	/// 
	/// let mut source = BufReader::new(File::open("file.txt")?);
	/// source.fill_buffer()?;
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn fill_buffer(&mut self) -> Result<&[u8]>;
	/// Clears the internal buffer.
	/// 
	/// # Example
	/// 
	/// ```no_run
	/// # use data_streams::Error;
	/// # #[cfg(feature = "std")]
	/// # {
	/// use std::{fs::File, io::BufReader};
	/// use data_streams::BufferAccess;
	///
	/// let mut source = BufReader::new(File::open("file.txt")?);
	/// source.fill_buffer()?;
	/// 
	/// source.clear_buffer();
	/// assert_eq!(source.buffer_count(), 0);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
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
	///
	/// # Example
	///
	/// ```no_run
	/// # use data_streams::Error;
	/// # #[cfg(feature = "std")]
	/// # {
	/// use std::{fs::File, io::BufReader};
	/// use data_streams::BufferAccess;
	///
	/// let mut source = BufReader::new(File::open("file.txt")?);
	/// source.fill_buffer()?;
	///
	/// source.drain_buffer(512);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
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
		buf_read_bytes(
			self,
			buf,
			<[u8]>::is_empty,
			|mut source, buf|
				source.read_bytes(buf).map(<[u8]>::len)
		)
	}

	default fn read_exact_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
		buf_read_exact_bytes(self, buf)
	}

	/// Reads bytes into a slice in multiples of `alignment`, returning the bytes
	/// read. This method is greedy; it consumes as many bytes as it can, until
	/// `buf` is filled or less than `alignment` bytes could be read.
	///
	/// If the alignment is zero or `buf`'s length is less than the alignment, the returned slice is
	/// empty.
	///
	/// # Errors
	///
	/// Returns any IO errors encountered.
	/// 
	/// [`Error::InsufficientBuffer`] is returned without reading if the buffer [capacity] is not
	/// large enough to hold at least one `alignment` width.
	/// 
	/// [capacity]: Self::buffer_capacity
	default fn read_aligned_bytes<'a>(&mut self, buf: &'a mut [u8], alignment: usize) -> Result<&'a [u8]> {
		if alignment == 0 { return Ok(&[]) }
		if self.buffer_capacity() < alignment {
			let spare_capacity = self.buffer_capacity() - self.buffer_count();
			return Err(Error::InsufficientBuffer {
				spare_capacity,
				required_count: alignment
			})
		}
		
		let len = buf.len() / alignment * alignment;
		buf_read_bytes(
			self,
			&mut buf[..len],
			|buf| buf.len() < alignment,
			|mut source, buf|
				source.read_aligned_bytes(buf, alignment).map(<[u8]>::len)
		)
	}

	#[cfg(feature = "utf8")]
	default fn read_utf8<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a str> {
		let mut valid_len = 0;
		let slice = buf_read_bytes(
			self,
			buf,
			<[u8]>::is_empty,
			|mut source, buf|
				match source.read_utf8(buf) {
					Ok(str) => {
						let len = str.len();
						valid_len += len;
						Ok(len)
					}
					Err(Error::Utf8(error)) =>
						Err(error.with_offset(valid_len).into()),
					Err(error) => Err(error)
				}
		)?;

		// Safety: valid_len bytes have been validated as UTF-8.
		Ok(unsafe { core::str::from_utf8_unchecked(slice) })
	}

	#[cfg(feature = "utf8")]
	default fn read_utf8_codepoint(&mut self, buf: &mut [u8; 4]) -> Result<char> {
		let str = match self.buffer() {
			&[first_byte, ..] => {
				let char_width = utf8_char_width(first_byte);
				self.read_utf8(&mut buf[..char_width])?
			},
			[] => default_read_utf8_codepoint(self, buf)?
		};
		
		Ok(str.parse().expect("bytes read by `read_utf8` must be valid UTF-8 codepoints"))
	}

	#[cfg(feature = "unstable_ascii_char")]
	default fn read_ascii<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [ascii::Char]> {
		default_read_ascii(self, buf)
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

/// Returns the maximum multiple of `factor` less than or equal to `value`.
pub(crate) const fn max_multiple_of(value: usize, factor: usize) -> usize {
	// For powers of 2, this optimizes to a simple AND of the negative factor.
	value / factor * factor
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
	assert_eq!(
		source.available(),
		avail.saturating_sub(count),
		"`drain_buffer` must remove buffered bytes"
	);
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

#[allow(clippy::panic, reason = "can't use assert here")]
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

fn default_read_aligned_bytes<'a>(source: &mut (impl DataSource + ?Sized), buf: &'a mut [u8], alignment: usize) -> Result<&'a [u8]> {
	if alignment == 0 {
		return Ok(&[])
	}
	
	let len = max_multiple_of(buf.len(), alignment);
	let mut slice = &mut buf[..len];
	let mut count = 0;
	while !slice.is_empty() && source.request(alignment)? {
		let avail = slice.len().min(max_multiple_of(source.available(), alignment));
		source.read_exact_bytes(&mut slice[..avail])?;
		count += avail;
		slice = &mut slice[avail..];
	}
	
	Ok(&buf[..count])
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

#[cfg(feature = "unstable_specialization")]
fn buf_read_bytes<'a>(
	source: &mut (impl BufferAccess + ?Sized),
	buf: &'a mut [u8],
	mut is_empty: impl FnMut(&[u8]) -> bool,
	mut slice_read_bytes: impl FnMut(&[u8], &mut [u8]) -> Result<usize>,
) -> Result<&'a [u8]> {
	let mut slice = &mut *buf;
	while !is_empty(slice) {
		let buf = match source.request(slice.len()) {
			Ok(_) => source.buffer(),
			Err(Error::InsufficientBuffer { .. }) => source.fill_buffer()?,
			Err(error) => return Err(error)
		};
		if is_empty(buf) {
			break
		}

		let count = slice_read_bytes(buf, slice)?;
		source.drain_buffer(count);
		slice = &mut slice[count..];
	}

	let unfilled = slice.len();
	let filled = buf.len() - unfilled;
	Ok(&buf[..filled])
}

#[cfg(all(feature = "alloc", feature = "utf8"))]
#[allow(dead_code, clippy::multiple_unsafe_ops_per_block)]
pub(crate) fn default_read_utf8<'a>(
	source: &mut (impl DataSource + ?Sized),
	count: usize,
	buf: &'a mut alloc::string::String
) -> Result<&'a str> {
	buf.reserve(count);
	// Safety: this function only modifies the string's bytes if the new bytes are found to be
	//  valid UTF-8.
	unsafe {
		append_utf8(buf, |b| {
			let len = b.len();
			b.set_len(len + count);
			source.read_bytes(&mut b[len..])
				  .map(<[u8]>::len)
		})
	}
}

#[cfg(feature = "utf8")]
fn default_read_utf8_codepoint<'a>(source: &mut (impl DataSource + ?Sized), buf: &'a mut [u8; 4]) -> Result<&'a str> {
	let (first_byte, remaining) = buf.split_at_mut(1);
	source.read_exact_bytes(first_byte)?;
	let char_width = utf8_char_width(first_byte[0]);
	source.read_exact_bytes(&mut remaining[..char_width - 1])?;
	Ok(from_utf8(&buf[..char_width])?)
}

#[cfg(feature = "unstable_ascii_char")]
fn default_read_ascii<'a>(source: &mut (impl DataSource + ?Sized), buf: &'a mut [u8]) -> Result<&'a [ascii::Char]> {
	let bytes = source.read_bytes(buf)?;
	let idx = count_ascii(bytes);
	if idx == bytes.len() {
		// Safety: all bytes have been checked as valid ASCII.
		Ok(unsafe { bytes.as_ascii_unchecked() })
	} else {
		Err(Error::invalid_ascii(bytes[idx], idx, bytes.len()))
	}
}

#[cfg(feature = "unstable_ascii_char")]
pub(crate) fn count_ascii(slice: &[u8]) -> usize {
	if slice.is_ascii() {
		slice.len()
	} else {
		// Safety: is_ascii indicates there is a non-ASCII character somewhere.
		unsafe { slice.iter().rposition(|b| !b.is_ascii()).unwrap_unchecked() }
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
			// Safety: exactly `len` bytes have been written.
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

#[cfg(all(
	test,
	feature = "std",
	feature = "alloc",
))]
mod read_aligned_test {
	use proptest::arbitrary::any;
	use proptest::collection::vec;
	use proptest::{prop_assert_eq, prop_assume, proptest};
	use crate::DataSource;

	proptest! {
		#[test]
		fn read_aligned(source in vec(any::<u8>(), 16..=256), alignment in 1usize..=16) {
			let buf = &mut [0; 256][..source.len()];
			let bytes = (&source[..]).read_aligned_bytes(buf, alignment).unwrap();
			prop_assert_eq!(bytes.len() % alignment, 0);
		}
	}
	
	proptest! {
		#[test]
		fn read_aligned_truncated(buf_size in 0usize..=15, alignment in 1usize..=16) {
			prop_assume!(buf_size < alignment);
			let buf = &mut [0; 15][..buf_size];
			let bytes = (&[0; 16][..]).read_aligned_bytes(buf, alignment).unwrap();
			prop_assert_eq!(bytes.len(), 0);
		}
	}
}
