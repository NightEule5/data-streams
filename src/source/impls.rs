// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

#![cfg(all(feature = "alloc", feature = "unstable_specialization"))]

use core::mem::MaybeUninit;
use crate::BufferAccess;
#[cfg(feature = "utf8")]
use alloc::string::String;
use alloc::vec::Vec;
use super::{DataSource, markers::SourceSize, Result};

#[cfg(feature = "utf8")]
pub fn buf_read_utf8_to_end<'a>(source: &mut impl BufferAccess, buf: &'a mut String) -> Result<&'a str> {
	unsafe {
		super::append_utf8(buf, |buf|
			buf_read_to_end(source, buf).map(<[u8]>::len)
		)
	}
}

pub fn buf_read_to_end<'a>(source: &mut impl BufferAccess, buf: &'a mut Vec<u8>) -> Result<&'a [u8]> {
	let start = buf.len();
	// Drain then bypass the buffer. We'll use the vector as a buffer instead.
	while source.buffer_count() > 0 {
		let buffer = source.buffer();
		buf.extend_from_slice(buffer);
		source.drain_buffer(buffer.len());
	}

	// If the buffer is larger than the default chunk size (8KiB), we'll use that
	// size instead.
	let chunk_size = source.buffer_capacity() as u64;
	read_to_end(source.bypass_buffer(), buf, chunk_size)?;
	Ok(&buf[start..])
}

// Reimplementation of std::io::default_read_to_end
pub fn read_to_end<'a>(source: &mut (impl DataSource + ?Sized), buf: &'a mut Vec<u8>, min_chunk_size: u64) -> Result<&'a [u8]> {
	trait SizeHint {
		fn size_hint(&self) -> Option<u64>;
	}

	impl<T: ?Sized> SizeHint for T {
		default fn size_hint(&self) -> Option<u64> { None }
	}

	impl<T: SourceSize + ?Sized> SizeHint for T {
		fn size_hint(&self) -> Option<u64> {
			self.upper_bound()
		}
	}

	const CHUNK_SIZE: u64 = if cfg!(target_os = "espidf") { 512 } else { 8 * 1024 };
	const PROBE_SIZE: usize = 32;

	fn probe(source: &mut (impl DataSource + ?Sized), buf: &mut Vec<u8>) -> Result<bool> {
		let probe = &mut [0; PROBE_SIZE];
		let bytes = source.read_bytes(probe)?;
		buf.extend_from_slice(bytes);
		Ok(!bytes.is_empty())
	}

	let start_len = buf.len();
	let start_cap = buf.capacity();
	let size_hint = source.size_hint();

	if matches!(size_hint, None | Some(0)) &&
		start_cap - start_len < PROBE_SIZE &&
		!probe(source, buf)? {
		return Ok(&[])
	}

	let mut initialized = 0;
	let mut chunk_size = size_hint.unwrap_or(min_chunk_size.max(CHUNK_SIZE));
	loop {
		if buf.len() == buf.capacity() && buf.capacity() == start_cap && !probe(source, buf)? {
			break Ok(&buf[start_len..])
		}

		if buf.len() == buf.capacity() {
			buf.try_reserve(PROBE_SIZE)?;
		}

		let mut spare = buf.spare_capacity_mut();
		let buf_len = spare.len().min(chunk_size as usize);
		spare = &mut spare[..buf_len];

		spare[initialized..].fill(MaybeUninit::new(0));
		let spare_init = unsafe {
			// Safety: all uninitialized bytes have been initialized above, and
			// MaybeUninit<u8> has the same layout as u8.
			&mut *(core::ptr::from_mut::<[MaybeUninit<u8>]>(spare) as *mut [u8]) // Stable slice_assume_init_ref
		};

		let read = source.read_bytes(spare_init)?.len();
		let empty_init = buf_len - read;

		if read == 0 {
			break Ok(&buf[start_len..])
		}

		initialized = empty_init;

		// Safety: this length was explicitly initialized above.
		unsafe {
			buf.set_len(read + buf.len());
		}
		
		// No size was provided. Bump up the read size if the source completely
		// fills the buffer.
		if size_hint.is_none() {
			// The source filled the buffer completely. Bump up the next buffer size.
			if buf_len as u64 >= chunk_size && read == buf_len {
				chunk_size = chunk_size.saturating_mul(2);
			}
		}
	}
}
