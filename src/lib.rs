// SPDX-License-Identifier: Apache-2.0

//! This crate provides stream traits for conveniently read and writing many data types: bytes,
//! little or big-endian integers, and UTF-8 strings. [`DataSource`] reads from a stream, [`DataSink`]
//! writes to a stream.
//!
//! Implementations for byte slices and `std::io`'s buffered readers and writers are provided, but
//! it's easy to write your own implementations:
//!
//! ```ignore
//! # use data_streams::{DataSource, DataSink, Result};
//!
//! struct MySource {
//!     buffer: Vec<u8>,
//!     // ...
//! }
//!
//! impl DataSource for MySource {
//!     fn available(&self) -> usize {
//!         self.buffer.len()
//!     }
//!
//!     fn request(&mut self, count: usize) -> Result<bool> {
//!         if self.available() < count {
//!             // Fill the buffer...
//!         }
//!
//!         Ok(self.available() >= count)
//!     }
//!
//!     fn read_bytes<'a>(&mut self, buf: &'a mut [u8]) -> Result<&'a [u8]> {
//!         let count = self.available().min(buf.len());
//!         buf[..count].copy_from_slice(&self.buffer);
//!         self.buffer.drain(..count);
//!         Ok(&buf[..count])
//!     }
//!
//!     fn read_utf8_to_end<'a>(&mut self, buf: &'a mut String) -> Result<&'a str> {
//!         self.read_utf8(self.available(), buf)
//!     }
//! }
//!
//! struct MySink {
//!     buffer: Vec<u8>,
//!     // ...
//! }
//!
//! impl DataSink for MySink {
//!     fn write_bytes(&mut self, buf: &[u8]) -> Result {
//!         self.buffer.extend_from_slice(buf);
//!         // Flush the buffer?
//!         Ok(())
//!     }
//! }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate core;
#[cfg(feature = "alloc")]
extern crate alloc;

mod slice;
mod std_io;
mod source;
mod sink;

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};
use core::fmt;
#[cfg(feature = "std")]
use std::io;
#[cfg(feature = "alloc")]
use simdutf8::compat::Utf8Error;
pub use sink::DataSink;
pub use source::DataSource;

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
