//! Adapts GPUI's exact Taffy 0.9.0 requirement to upstream's 0.9.2 maintenance
//! release. Remove this facade when GPUI accepts the maintained dependency.

#![cfg_attr(not(feature = "std"), no_std)]

pub use taffy_upstream::*;
