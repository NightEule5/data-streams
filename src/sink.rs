// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

use num_traits::PrimInt;
use bytemuck::{bytes_of, Pod};
use crate::Result;

/// A sink stream of data.
pub trait DataSink {
	/// Writes all bytes from `buf`. Equivalent to [`Write::write_all`].
	/// 
	/// [`Write::write_all`]: io::Write::write_all
	fn write_bytes(&mut self, buf: &[u8]) -> Result;

	/// Writes a [`u8`].
	fn write_u8(&mut self, value: u8) -> Result { self.write_int_be_spec(value) }
	/// Writes an [`i8`].
	fn write_i8(&mut self, value: i8) -> Result { self.write_int_be_spec(value) }
	/// Writes a big-endian [`u16`].
	fn write_u16(&mut self, value: u16) -> Result { self.write_int_be_spec(value) }
	/// Writes a big-endian [`i16`].
	fn write_i16(&mut self, value: i16) -> Result { self.write_int_be_spec(value) }
	/// Writes a little-endian [`u16`].
	fn write_u16_le(&mut self, value: u16) -> Result { self.write_int_le_spec(value) }
	/// Writes a little-endian [`i16`].
	fn write_i16_le(&mut self, value: i16) -> Result { self.write_int_le_spec(value) }
	/// Writes a big-endian [`u32`].
	fn write_u32(&mut self, value: u32) -> Result { self.write_int_be_spec(value) }
	/// Writes a big-endian [`i32`].
	fn write_i32(&mut self, value: i32) -> Result { self.write_int_be_spec(value) }
	/// Writes a little-endian [`u32`].
	fn write_u32_le(&mut self, value: u32) -> Result { self.write_int_le_spec(value) }
	/// Writes a little-endian [`i32`].
	fn write_i32_le(&mut self, value: i32) -> Result { self.write_int_le_spec(value) }
	/// Writes a big-endian [`u64`].
	fn write_u64(&mut self, value: u64) -> Result { self.write_int_be_spec(value) }
	/// Writes a big-endian [`i64`].
	fn write_i64(&mut self, value: i64) -> Result { self.write_int_be_spec(value) }
	/// Writes a little-endian [`u64`].
	fn write_u64_le(&mut self, value: u64) -> Result { self.write_int_le_spec(value) }
	/// Writes a little-endian [`i64`].
	fn write_i64_le(&mut self, value: i64) -> Result { self.write_int_le_spec(value) }
	/// Writes a big-endian [`u128`].
	fn write_u128(&mut self, value: u128) -> Result { self.write_int_be_spec(value) }
	/// Writes a big-endian [`i128`].
	fn write_i128(&mut self, value: i128) -> Result { self.write_int_be_spec(value) }
	/// Writes a little-endian [`u128`].
	fn write_u128_le(&mut self, value: u128) -> Result { self.write_int_le_spec(value) }
	/// Writes a little-endian [`i128`].
	fn write_i128_le(&mut self, value: i128) -> Result { self.write_int_le_spec(value) }
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
	fn write_int<T: PrimInt + Pod>(&mut self, value: T) -> Result
	where Self: Sized {
		self.write_int_be_spec(value)
	}
	/// Writes a little-endian integer.
	fn write_int_le<T: PrimInt + Pod>(&mut self, value: T) -> Result
	where Self: Sized {
		self.write_int_le_spec(value)
	}
	/// Writes a value of an arbitrary bit pattern. See [`Pod`].
	fn write_data<T: Pod>(&mut self, value: T) -> Result
	where Self: Sized {
		self.write_data_spec(value)
	}
	/// Writes a UTF-8 string.
	fn write_utf8(&mut self, value: &str) -> Result {
		self.write_bytes(value.as_bytes())
	}
}

trait WriteSpec<T: Pod>: DataSink {
	fn write_int_be_spec(&mut self, value: T) -> Result
	where T: PrimInt {
		self.write_data_spec(value.to_be())
	}
	fn write_int_le_spec(&mut self, value: T) -> Result
	where T: PrimInt {
		self.write_data_spec(value.to_le())
	}
	fn write_data_spec(&mut self, value: T) -> Result {
		self.write_bytes(bytes_of(&value))
	}
}

impl<S: DataSink + ?Sized, T: Pod> WriteSpec<T> for S { }
