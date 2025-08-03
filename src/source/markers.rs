// Copyright 2024 - Strixpyrr
// SPDX-License-Identifier: Apache-2.0

use crate::DataSource;

/// A trait which marks a source as infinite, preventing "read-to-end" operations
/// from completing.
/// 
/// # Safety
/// 
/// The source must be truly infinite; it must **always** produce data. An example
/// in the standard library is [`std::io::Repeat`].
pub unsafe trait InfiniteSource: DataSource { }

/// A trait which gives known upper and lower bounds of the size of the source.
/// 
/// # Safety
/// 
/// Other functions rely on these bounds being correct. The source must produce at
/// least the number of bytes returned by [`lower_bound`], and at most that returned
/// by [`upper_bound`].
/// 
/// [`lower_bound`]: SourceSize::lower_bound
/// [`upper_bound`]: SourceSize::upper_bound
pub unsafe trait SourceSize {
	fn lower_bound(&self) -> u64 { 0 }
	fn upper_bound(&self) -> Option<u64> { None }
}

// Safety: infinite sources produce at least zero bytes and have no upper bound, by definition.
unsafe impl<T: InfiniteSource> SourceSize for T { }
