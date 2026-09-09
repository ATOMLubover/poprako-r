#![deny(clippy::correctness)]
#![deny(clippy::suspicious)]
#![deny(clippy::complexity)]
#![deny(clippy::perf)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]
#![deny(clippy::exit)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::string_slice)]
#![deny(clippy::mod_module_files)]
#![warn(clippy::style)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::future_not_send)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::uninlined_format_args)]

//! Direct macros for static `ObjDept` composition.

// Expands one typed object declaration.
mod object;
// Parses shared total-department object manifest entries.
mod obj_dept_entry;
// Expands one total ObjDept implementation.
mod impl_obj_dept;
// Expands one typed RDB ObjDeptProm adapter.
mod rdb_obj_dept_prom;

#[cfg(test)]
mod tests;

use proc_macro::TokenStream;

/// Declares the complete object manifest and typed Diesel operations.
#[proc_macro]
pub fn objs_def(input: TokenStream) -> TokenStream {
    //
    object::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Declares one typed Diesel `ObjDeptProm` adapter.
#[proc_macro]
pub fn rdb_obj_dept_prom(input: TokenStream) -> TokenStream {
    //
    rdb_obj_dept_prom::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implements one total `ObjDept` for its static object set.
#[proc_macro]
pub fn impl_obj_dept(input: TokenStream) -> TokenStream {
    //
    impl_obj_dept::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Expands ObjDept items supplied by the local object manifest.
#[doc(hidden)]
#[proc_macro]
pub fn expand_obj_dept_items(input: TokenStream) -> TokenStream {
    //
    impl_obj_dept::expand_items(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
