// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(all(feature = "alloc", feature = "utf8"))]
use alloc::string::String;
#[cfg(feature = "unstable_ascii_char")]
use core::ascii;
use num_traits::PrimInt;
use bytemuck::{bytes_of, Pod};
use crate::Result;

/// A sink stream of data.
pub trait DataSink {
	/// Writes all bytes from `buf`. Equivalent to [`Write::write_all`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// [`Write::write_all`]: io::Write::write_all
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_bytes(b"Hello!")?;
	/// assert_eq!(buf, b"Hello!");
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_bytes(&mut self, buf: &[u8]) -> Result;
	/// Writes a UTF-8 string.
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_utf8("Hello!")?;
	/// assert_eq!(buf, b"Hello!");
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_utf8(&mut self, value: &str) -> Result {
		self.write_bytes(value.as_bytes())
	}
	/// Writes a single UTF-8 codepoint.
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_utf8_codepoint('🍉')?;
	/// assert_eq!(buf, "🍉".as_bytes());
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_utf8_codepoint(&mut self, value: char) -> Result {
		let mut buf = [0; 4];
		self.write_utf8(value.encode_utf8(&mut buf))
	}
	/// Writes an ASCII slice.
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// #![feature(ascii_char)]
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_ascii("Hello!".as_ascii().unwrap());
	/// assert_eq!(buf, b"Hello!");
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	#[cfg(feature = "unstable_ascii_char")]
	fn write_ascii(&mut self, value: &[ascii::Char]) -> Result {
		self.write_bytes(value.as_bytes())
	}

	/// Writes a [`u8`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_u8(127)?;
	/// assert_eq!(buf, [127]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_u8(&mut self, value: u8) -> Result { self.write_data(value) }
	/// Writes an [`i8`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_i8(-127)?;
	/// assert_eq!(buf, [-127i8 as u8]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_i8(&mut self, value: i8) -> Result { self.write_data(value) }
	/// Writes a big-endian [`u16`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_u16(0x1234)?;
	/// assert_eq!(buf, [0x12, 0x34]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_u16(&mut self, value: u16) -> Result { self.write_int(value) }
	/// Writes a big-endian [`i16`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_i16(0x1234)?;
	/// assert_eq!(buf, [0x12, 0x34]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_i16(&mut self, value: i16) -> Result { self.write_int(value) }
	/// Writes a little-endian [`u16`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_u16_le(0x1234)?;
	/// assert_eq!(buf, [0x34, 0x12]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_u16_le(&mut self, value: u16) -> Result { self.write_int_le(value) }
	/// Writes a little-endian [`i16`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_i16_le(0x1234)?;
	/// assert_eq!(buf, [0x34, 0x12]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_i16_le(&mut self, value: i16) -> Result { self.write_int_le(value) }
	/// Writes a big-endian [`u32`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_u32(0x12345678)?;
	/// assert_eq!(buf, [0x12, 0x34, 0x56, 0x78]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_u32(&mut self, value: u32) -> Result { self.write_int(value) }
	/// Writes a big-endian [`i32`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_i32(0x12345678)?;
	/// assert_eq!(buf, [0x12, 0x34, 0x56, 0x78]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_i32(&mut self, value: i32) -> Result { self.write_int(value) }
	/// Writes a little-endian [`u32`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_u32_le(0x12345678)?;
	/// assert_eq!(buf, [0x78, 0x56, 0x34, 0x12]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_u32_le(&mut self, value: u32) -> Result { self.write_int_le(value) }
	/// Writes a little-endian [`i32`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_i32_le(0x12345678)?;
	/// assert_eq!(buf, [0x78, 0x56, 0x34, 0x12]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_i32_le(&mut self, value: i32) -> Result { self.write_int_le(value) }
	/// Writes a big-endian [`u64`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_u64(0x1234_5678_9ABC_DEF0)?;
	/// assert_eq!(buf, [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_u64(&mut self, value: u64) -> Result { self.write_int(value) }
	/// Writes a big-endian [`i64`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_i64(0x1234_5678_9ABC_DEF0)?;
	/// assert_eq!(buf, [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_i64(&mut self, value: i64) -> Result { self.write_int(value) }
	/// Writes a little-endian [`u64`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_u64_le(0x1234_5678_9ABC_DEF0)?;
	/// assert_eq!(buf, [0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_u64_le(&mut self, value: u64) -> Result { self.write_int_le(value) }
	/// Writes a little-endian [`i64`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_i64_le(0x1234_5678_9ABC_DEF0)?;
	/// assert_eq!(buf, [0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_i64_le(&mut self, value: i64) -> Result { self.write_int_le(value) }
	/// Writes a big-endian [`u128`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_u128(0x1234_5678_9ABC_DEF0_0FED_CBA9_8765_4321)?;
	/// assert_eq!(buf, [
	///     0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0,
	///     0x0F, 0xED, 0xCB, 0xA9, 0x87, 0x65, 0x43, 0x21
	/// ]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_u128(&mut self, value: u128) -> Result { self.write_int(value) }
	/// Writes a big-endian [`i128`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_i128(0x1234_5678_9ABC_DEF0_0FED_CBA9_8765_4321)?;
	/// assert_eq!(buf, [
	///     0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0,
	///     0x0F, 0xED, 0xCB, 0xA9, 0x87, 0x65, 0x43, 0x21
	/// ]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_i128(&mut self, value: i128) -> Result { self.write_int(value) }
	/// Writes a little-endian [`u128`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_u128_le(0x1234_5678_9ABC_DEF0_0FED_CBA9_8765_4321)?;
	/// assert_eq!(buf, [
	///     0x21, 0x43, 0x65, 0x87, 0xA9, 0xCB, 0xED, 0x0F,
	///     0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12
	/// ]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_u128_le(&mut self, value: u128) -> Result { self.write_int_le(value) }
	/// Writes a little-endian [`i128`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_i128_le(0x1234_5678_9ABC_DEF0_0FED_CBA9_8765_4321)?;
	/// assert_eq!(buf, [
	///     0x21, 0x43, 0x65, 0x87, 0xA9, 0xCB, 0xED, 0x0F,
	///     0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12
	/// ]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_i128_le(&mut self, value: i128) -> Result { self.write_int_le(value) }
	/// Writes a big-endian [`usize`]. To make streams consistent across platforms,
	/// [`usize`] is fixed to the size of [`u64`] regardless of the target platform.
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(all(feature = "alloc", target_pointer_width = "64"))]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_usize(0x1234_5678_9ABC_DEF0)?;
	/// assert_eq!(buf, [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_usize(&mut self, value: usize) -> Result {
		self.write_u64(value as u64)
	}
	/// Writes a big-endian [`isize`]. To make streams consistent across platforms,
	/// [`isize`] is fixed to the size of [`i64`] regardless of the target platform.
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(all(feature = "alloc", target_pointer_width = "64"))]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_isize(0x1234_5678_9ABC_DEF0)?;
	/// assert_eq!(buf, [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_isize(&mut self, value: isize) -> Result {
		self.write_i64(value as i64)
	}
	/// Writes a little-endian [`usize`]. To make streams consistent across platforms,
	/// [`usize`] is fixed to the size of [`u64`] regardless of the target platform.
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(all(feature = "alloc", target_pointer_width = "64"))]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_usize_le(0x1234_5678_9ABC_DEF0)?;
	/// assert_eq!(buf, [0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_usize_le(&mut self, value: usize) -> Result {
		self.write_u64_le(value as u64)
	}
	/// Writes a little-endian [`isize`]. To make streams consistent across platforms,
	/// [`isize`] is fixed to the size of [`i64`] regardless of the target platform.
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(all(feature = "alloc", target_pointer_width = "64"))]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::DataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_isize_le(0x1234_5678_9ABC_DEF0)?;
	/// assert_eq!(buf, [0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_isize_le(&mut self, value: isize) -> Result {
		self.write_i64_le(value as i64)
	}
}

/// Writes generic data to a [sink](DataSink).
pub trait GenericDataSink: DataSink {
	/// Writes a big-endian integer.
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::GenericDataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_int(0x12345678)?;
	/// assert_eq!(buf, [0x12, 0x34, 0x56, 0x78]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_int<T: Pod + PrimInt>(&mut self, value: T) -> Result {
		self.write_data(value.to_be())
	}
	/// Writes a little-endian integer.
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(feature = "alloc")]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::GenericDataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_int_le(0x12345678)?;
	/// assert_eq!(buf, [0x78, 0x56, 0x34, 0x12]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_int_le<T: Pod + PrimInt>(&mut self, value: T) -> Result {
		self.write_data(value.to_le())
	}
	/// Writes a value of an arbitrary bit pattern. See [`Pod`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Example
	///
	/// ```
	/// # use data_streams::Error;
	/// # extern crate alloc;
	/// # #[cfg(all(feature = "alloc", target_endian = "little"))]
	/// # {
	/// # use alloc::vec::Vec;
	/// use data_streams::GenericDataSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_data(0x12345678)?;
	/// assert_eq!(buf, [0x78, 0x56, 0x34, 0x12]);
	/// # }
	/// # Ok::<_, Error>(())
	/// ```
	fn write_data<T: Pod>(&mut self, value: T) -> Result {
		self.write_bytes(bytes_of(&value))
	}
}

impl<S: DataSink + ?Sized> GenericDataSink for S { }

/// A sink stream of vector data.
#[cfg(feature = "alloc")]
pub trait VecSink: DataSink {
	/// Writes all bytes from a [`Vec`].
	/// 
	/// # Errors
	/// 
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Examples
	///
	/// ```
	/// # extern crate alloc;
	/// # use alloc::vec::Vec;
	/// # use data_streams::Error;
	/// use data_streams::VecSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_owned_bytes(b"Hello!".into())?;
	/// assert_eq!(buf, b"Hello!");
	/// # Ok::<_, Error>(())
	/// ```
	///
	/// # Implementation
	/// 
	/// By default, this delegates to [`write_bytes`]. Some implementations may be
	/// may better optimize for owned data.
	/// 
	/// [`write_bytes`]: DataSink::write_bytes
	fn write_owned_bytes(&mut self, buf: Vec<u8>) -> Result;
	/// Writes all UTF-8 bytes from a [`String`].
	///
	/// # Errors
	///
	/// May return [`Overflow`](Error::Overflow) if the sink would exceed some hard
	/// storage limit. In the case, the stream is filled completely, excluding the
	/// overflowing bytes.
	///
	/// # Examples
	///
	/// ```
	/// # extern crate alloc;
	/// # use alloc::vec::Vec;
	/// # use data_streams::Error;
	/// use data_streams::VecSink;
	///
	/// let mut buf = Vec::new();
	/// buf.write_owned_utf8("Hello!".into())?;
	/// assert_eq!(buf, b"Hello!");
	/// # Ok::<_, Error>(())
	/// ```
	///
	/// # Implementation
	///
	/// By default, this delegates to [`write_utf8`]. Some implementations may be
	/// may better optimize for owned data.
	///
	/// [`write_utf8`]: DataSink::write_utf8
	#[cfg(feature = "utf8")]
	fn write_owned_utf8(&mut self, buf: String) -> Result;
}

#[cfg(all(feature = "alloc", feature = "unstable_specialization"))]
impl<T: DataSink> VecSink for T {
	default fn write_owned_bytes(&mut self, buf: Vec<u8>) -> Result {
		self.write_bytes(&buf)
	}

	#[cfg(feature = "utf8")]
	default fn write_owned_utf8(&mut self, buf: String) -> Result {
		self.write_utf8(&buf)
	}
}

#[cfg(all(feature = "alloc", not(feature = "unstable_specialization")))]
impl<T: DataSink> VecSink for T {
	fn write_owned_bytes(&mut self, buf: Vec<u8>) -> Result {
		self.write_bytes(&buf)
	}

	#[cfg(feature = "utf8")]
	fn write_owned_utf8(&mut self, buf: String) -> Result {
		self.write_utf8(&buf)
	}
}
