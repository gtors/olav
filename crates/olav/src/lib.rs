//! `olav` — compile-time XML template engine, maud-inspired.
//!
//! # Example
//!
//! ```
//! use olav::xml;
//!
//! let name = "world";
//! let doc = xml! {
//!     root {
//!         greeting { "hello " @name }
//!     }
//! };
//! assert_eq!(doc.as_str(), "<root><greeting>hello world</greeting></root>");
//! ```

pub mod escape;
pub mod markup;
pub mod pre_escaped;
pub mod render;

pub use markup::Markup;
pub use pre_escaped::PreEscaped;
pub use render::{DisplayValue, Render};

pub use olav_macros::xml;
