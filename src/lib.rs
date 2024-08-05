// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

//! This crate provides stream traits for conveniently read and writing many data types: bytes,
//! little or big-endian integers, and UTF-8 strings. [`DataSource`] reads from a stream, [`DataSink`]
//! writes to a stream.
//!
//! Implementations for byte slices and `std::io`'s buffered readers and writers are provided, but
//! it's easy to write your own implementations:
//!
//! ```no_run
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
//!     fn skip(&mut self, count: usize) -> Result<usize> {
//!         // Read bytes up to count bytes from the stream...
//!         // Here we just consume from the buffer as an example.
//!         self.buffer.drain(..count);
//!         Ok(count)
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
//! 
//! # Feature flags
//! 
//! - `std`: Provides impls for [`std::io`] types, such as [`BufReader`](std::io::BufReader) and
//!   [`BufWriter`](std::io::BufWriter). Requires a dependency on the Rust standard library. Disable
//!   to allow usage with `no_std`.
//! - `alloc`: Provides impls for dynamically allocated types such as [`Vec`], and source methods
//!   for reading into these. Requires a heap allocator, which may not be present on platforms
//!   without the standard library.
//! - `unstable`: Provides unstable features only present on the nightly compiler. Enables:
//!   - `unstable_borrowed_buf`: Provides [`DataSource`] impls for [`BorrowedBuf`](core::io::BorrowedBuf)
//!     and [`BorrowedCursor`](core::io::BorrowedCursor).
//!   - `unstable_specialization`: Enables trait specialization, providing a default [`DataSource`]
//!     for impls of [`BufferAccess`].
//!   - `unstable_uninit_slice`: Provides a [`DataSink`] impl for `&mut [MaybeUninit<u8>]`.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "unstable_specialization", feature(specialization))]
#![cfg_attr(feature = "unstable_borrowed_buf", feature(core_io_borrowed_buf))]
#![cfg_attr(feature = "unstable_uninit_slice", feature(maybe_uninit_write_slice))]
#![cfg_attr(test, feature(assert_matches))]
#![allow(incomplete_features)]

#![deny(clippy::pedantic)]
#![allow(
	clippy::cast_sign_loss, // I know
	clippy::cast_possible_truncation, // Yes, and?
	clippy::module_name_repetitions,
	clippy::must_use_candidate,
)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod error;
mod source;
mod sink;
mod slice;
mod vec;
mod core_io;
mod std_io;
mod wrappers;

pub use error::Error;
pub use sink::{DataSink, GenericDataSink};
pub use source::{BufferAccess, DataSource, GenericDataSource};

pub type Result<T = (), E = Error> = core::result::Result<T, E>;
