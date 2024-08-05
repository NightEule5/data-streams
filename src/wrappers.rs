// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

#[cfg(all(feature = "alloc", not(feature = "unstable_specialization")))]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
use crate::{BufferAccess, DataSink, Result};
#[cfg(not(feature = "unstable_specialization"))]
use crate::{DataSource, source::default_read_array};

// Todo: DataSource couldn't be implemented for &mut <source> when specialization
//  is enabled.

// This was a PITA to get working. Did this save much time? No idea
macro_rules! delegate_impl {
    (with $reduced:expr;
	$(
	fn $name:ident($($params:tt)+)$( -> $ret:ty)?;
	)+) => {
		$(fn $name($($params)+)$( -> $ret)? {
			delegate_impl!(@$reduced;$name($($params)+))
		})+
	};
	(@$reduced:expr;$name:ident(&$(mut)? self$(, $param:ident: $param_ty:ty)*)) => {
		$reduced.$name($($param),*)
	};
}

macro_rules! impl_buf_access {
    ($($(#[$attr:meta])? impl<$gen:ident> for $ty:ty;)+) => {
		$(
		$(#[$attr])?
		impl<$gen: BufferAccess + ?Sized> BufferAccess for $ty {
			delegate_impl! {
				with **self;
				fn buffer_capacity(&self) -> usize;
				fn buffer(&self) -> &[u8];
				fn fill_buffer(&mut self) -> Result<&[u8]>;
				fn clear_buffer(&mut self);
				fn drain_buffer(&mut self, count: usize);
			}
		})+
	};
}

impl_buf_access! {
	impl<S> for &mut S;
	#[cfg(feature = "alloc")]
	impl<S> for Box<S>;
}

macro_rules! impl_source {
    ($($(#[$attr:meta])? impl<$gen:ident> for $ty:ty;)+) => {
		$(
		$(#[$attr])?
		impl<$gen: DataSource + ?Sized> DataSource for $ty {
			delegate_impl! {
				with **self;
				fn available(&self) -> usize;
				fn request(&mut self, count: usize) -> Result<bool>;
				fn skip(&mut self, count: usize) -> Result<usize>;
				fn require(&mut self, count: usize) -> Result;
				fn read_u8(&mut self) -> Result<u8>;
				fn read_i8(&mut self) -> Result<i8>;
				fn read_u16(&mut self) -> Result<u16>;
				fn read_i16(&mut self) -> Result<i16>;
				fn read_u16_le(&mut self) -> Result<u16>;
				fn read_i16_le(&mut self) -> Result<i16>;
				fn read_u32(&mut self) -> Result<u32>;
				fn read_i32(&mut self) -> Result<i32>;
				fn read_u32_le(&mut self) -> Result<u32>;
				fn read_i32_le(&mut self) -> Result<i32>;
				fn read_u64(&mut self) -> Result<u64>;
				fn read_i64(&mut self) -> Result<i64>;
				fn read_u64_le(&mut self) -> Result<u64>;
				fn read_i64_le(&mut self) -> Result<i64>;
				fn read_u128(&mut self) -> Result<u128>;
				fn read_i128(&mut self) -> Result<i128>;
				fn read_u128_le(&mut self) -> Result<u128>;
				fn read_i128_le(&mut self) -> Result<i128>;
				fn read_usize(&mut self) -> Result<usize>;
				fn read_isize(&mut self) -> Result<isize>;
				fn read_usize_le(&mut self) -> Result<usize>;
				fn read_isize_le(&mut self) -> Result<isize>;
			}

			fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
				(**self).read_bytes(buf)
			}

			fn read_exact_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
				(**self).read_exact_bytes(buf)
			}

			fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
				default_read_array(&mut **self)
			}

			#[cfg(feature = "alloc")]
			fn read_utf8<'a>(&mut self, count: usize, buf: &'a mut String) -> Result<&'a str> {
				(**self).read_utf8(count, buf)
			}

			#[cfg(feature = "alloc")]
			fn read_utf8_to_end<'a>(&mut self, buf: &'a mut String) -> Result<&'a str> {
				(**self).read_utf8_to_end(buf)
			}
		})+
	};
}

impl_source! {
	// Conflicts with specialized impl, because outside crates are allowed to implement
	// this trait for mutable references of their type; &mut S is a "foreign type" in
	// compiler terms. I don't know a way around this issue, so we'll disable it when
	// specialization is enabled. Fixing this down the line shouldn't be a breaking change.
	#[cfg(not(feature = "unstable_specialization"))]
	impl<S> for &mut S;
	#[cfg(all(feature = "alloc", not(feature = "unstable_specialization")))]
	impl<S> for Box<S>;
}

macro_rules! impl_sink {
    ($($(#[$attr:meta])? impl<$gen:ident> for $ty:ty;)+) => {
		$(
		$(#[$attr])?
		impl<$gen: DataSink + ?Sized> DataSink for $ty {
			delegate_impl! {
				with **self;
				fn write_bytes(&mut self, buf: &[u8]) -> Result;
				fn write_utf8(&mut self, value: &str) -> Result;
				fn write_u8(&mut self, value: u8) -> Result;
				fn write_i8(&mut self, value: i8) -> Result;
				fn write_u16(&mut self, value: u16) -> Result;
				fn write_i16(&mut self, value: i16) -> Result;
				fn write_u16_le(&mut self, value: u16) -> Result;
				fn write_i16_le(&mut self, value: i16) -> Result;
				fn write_u32(&mut self, value: u32) -> Result;
				fn write_i32(&mut self, value: i32) -> Result;
				fn write_u32_le(&mut self, value: u32) -> Result;
				fn write_i32_le(&mut self, value: i32) -> Result;
				fn write_u64(&mut self, value: u64) -> Result;
				fn write_i64(&mut self, value: i64) -> Result;
				fn write_u64_le(&mut self, value: u64) -> Result;
				fn write_i64_le(&mut self, value: i64) -> Result;
				fn write_u128(&mut self, value: u128) -> Result;
				fn write_i128(&mut self, value: i128) -> Result;
				fn write_u128_le(&mut self, value: u128) -> Result;
				fn write_i128_le(&mut self, value: i128) -> Result;
				fn write_usize(&mut self, value: usize) -> Result;
				fn write_isize(&mut self, value: isize) -> Result;
				fn write_usize_le(&mut self, value: usize) -> Result;
				fn write_isize_le(&mut self, value: isize) -> Result;
			}
		}
		)+
	};
}

impl_sink! {
	impl<S> for &mut S;
	#[cfg(feature = "alloc")]
	impl<S> for Box<S>;
}
