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
//! - `utf8`: Enables reading UTF-8-validated data from sources, and writing to [`String`]s, using a
//!   very fast SIMD validation algorithm from the [`simdutf8`](https://github.com/rusticstuff/simdutf8)
//!   crate. UTF-8 can be written to sinks without this feature.
//! - `unstable`: Provides unstable features only present on the nightly compiler. Enables:
//!   - `unstable_borrowed_buf`: Provides [`DataSource`] impls for [`BorrowedBuf`](core::io::BorrowedBuf)
//!     and [`BorrowedCursor`](core::io::BorrowedCursor).
//!   - `unstable_specialization`: Enables trait specialization, providing a default [`DataSource`]
//!     for impls of [`BufferAccess`].
//!   - `unstable_uninit_slice`: Provides a [`DataSink`] impl for `&mut [MaybeUninit<u8>]`.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "unstable_ascii_char", feature(ascii_char))]
#![cfg_attr(feature = "unstable_specialization", feature(specialization))]
#![cfg_attr(feature = "unstable_borrowed_buf", feature(core_io_borrowed_buf))]
#![cfg_attr(feature = "unstable_uninit_slice", feature(maybe_uninit_write_slice))]
#![cfg_attr(test, feature(assert_matches))]
#![allow(incomplete_features)]

#![deny(
	clippy::alloc_instead_of_core,
	clippy::as_pointer_underscore,
	clippy::as_underscore,
	clippy::assertions_on_result_states,
	clippy::cfg_not_test,
	clippy::clone_on_ref_ptr,
	clippy::decimal_literal_representation,
	clippy::deref_by_slicing,
	clippy::else_if_without_else,
	clippy::empty_drop,
	clippy::empty_enum_variants_with_brackets,
	clippy::empty_structs_with_brackets,
	clippy::exhaustive_enums,
	clippy::field_scoped_visibility_modifiers,
	clippy::if_then_some_else_none,
	clippy::impl_trait_in_params,
	clippy::infinite_loop,
	clippy::map_err_ignore,
	clippy::mem_forget,
	clippy::missing_assert_message,
	clippy::missing_errors_doc,
	clippy::missing_panics_doc,
	clippy::missing_safety_doc,
	clippy::multiple_unsafe_ops_per_block,
	clippy::panic,
	clippy::partial_pub_fields,
	clippy::redundant_type_annotations,
	clippy::ref_patterns,
	clippy::renamed_function_params,
	clippy::semicolon_inside_block,
	clippy::std_instead_of_alloc,
	clippy::std_instead_of_core,
	clippy::undocumented_unsafe_blocks,
	clippy::unwrap_used,
)]

#[cfg(feature = "alloc")]
extern crate alloc;
extern crate core;

mod error;
mod source;
mod sink;
mod slice;
mod vec;
mod core_io;
mod std_io;
mod utf8;
mod wrappers;

pub mod markers {
	pub mod source {
		pub use crate::source::markers::{InfiniteSource, SourceSize};
	}
}

pub use error::Error;
#[cfg(feature = "unstable_ascii_char")]
pub use error::AsciiError;
#[cfg(feature = "utf8")]
pub use error::{Utf8Error, Utf8ErrorKind, SimdUtf8Error};
pub use sink::{DataSink, GenericDataSink};
#[cfg(feature = "alloc")]
pub use sink::VecSink;
pub use source::{BufferAccess, DataSource, GenericDataSource};
#[cfg(feature = "alloc")]
pub use source::VecSource;

pub type Result<T = (), E = Error> = core::result::Result<T, E>;
