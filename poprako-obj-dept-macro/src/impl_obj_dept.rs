// Generates object lifecycle implementations from the manifest.
mod lifecycle;
// Generates object read implementations from the manifest.
mod read;

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, Result, Token};

use crate::obj_dept_entry::ObjEntry;

// Stores the total-department manifest arguments.
struct DeptInput {
    //
    // Identifies the total object department type.
    dept: Ident,

    // Identifies the read-only object department type.
    view: Ident,
}

impl Parse for DeptInput {
    // Parses the total-department manifest arguments.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        //
        parse_field(input, "dept")?;

        let dept = input.parse()?;

        input.parse::<Token![,]>()?;

        parse_field(input, "view")?;

        let view = input.parse()?;

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }

        if !input.is_empty() {
            return Err(input.error("unexpected ObjDept implementation tokens"));
        }

        Ok(Self { dept, view })
    }
}

// Stores all arguments needed to expand department items.
struct ItemsInput {
    //
    // Identifies the total object department type.
    dept: Ident,

    // Identifies the read-only object department type.
    view: Ident,

    // Contains object manifest entries.
    entries: Punctuated<ObjEntry, Token![,]>,
}

impl Parse for ItemsInput {
    // Parses the expanded department item arguments.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        //
        parse_field(input, "dept")?;

        let dept = input.parse()?;

        input.parse::<Token![,]>()?;

        parse_field(input, "view")?;

        let view = input.parse()?;

        input.parse::<Token![;]>()?;

        let entries = input.parse_terminated(ObjEntry::parse, Token![,])?;

        Ok(Self {
            dept,
            view,
            entries,
        })
    }
}

/// Expands the total `ObjDept` operations and actor construction.
pub fn expand(input: TokenStream) -> Result<TokenStream> {
    //
    let DeptInput { dept, view } = syn::parse2(input)?;

    Ok(quote! {
        //
        macro_rules! implement_obj_dept_from_manifest {
            //
            ($($entry:tt)*) => {
                ::poprako_obj_dept::expand_obj_dept_items! {
                    dept: #dept,
                    view: #view;
                    $($entry)*
                }
            };
        }

        for_each_obj!(implement_obj_dept_from_manifest);
    })
}

/// Expands complete `ObjDept` items after the local manifest callback.
pub fn expand_items(input: TokenStream) -> Result<TokenStream> {
    //
    let ItemsInput {
        dept,
        view,
        entries,
    } = syn::parse2(input)?;

    let read_impls = entries
        .iter()
        .map(|entry| read::expand(&dept, &view, entry));

    let lifecycle_impls =
        entries.iter().map(|entry| lifecycle::expand(&dept, entry));

    let dispatch_arms = entries.iter().map(|entry| {
        //
        let obj = entry.marker();

        let module = entry.module();

        let topic = entry.topic();

        quote! {
            #topic => ::poprako_obj_dept::handle_obj_dept_task!(
                core,
                pool,
                task,
                #obj,
                #module,
            ),
        }
    });

    Ok(quote! {
        impl<P, M> #dept<P, M>
        where
            P: ::poprako_obj_dept::pool::ObjDeptPool + ::core::marker::Sync,
            M: ::poprako_obj_dept::prom::ObjDeptProm + ::core::marker::Sync,
        {
            /// Dispatches an object task using the injected storage dependencies.
            pub async fn dispatch(
                core: ::poprako_rdb_core::RdbCore,
                pool: P,
                task: ::poprako_obj_dept::model::task::ObjDeptPromTask,
            ) -> ::poprako_obj_dept::rest::ObjDeptRest<
                ::poprako_obj_dept::model::task::ObjTaskAction,
            > {
                match task.topic.as_str() {
                    #(#dispatch_arms)*
                    _ => Ok(
                        ::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                            message: "unknown object topic".into(),
                        },
                    ),
                }
            }
        }

        #(#read_impls)*
        #(#lifecycle_impls)*
    })
}

// Parses a named field in a macro input declaration.
fn parse_field(input: ParseStream<'_>, expected: &str) -> Result<()> {
    //
    let field = input.parse::<Ident>()?;

    if field != expected {
        //
        return Err(syn::Error::new(
            field.span(),
            format!("expected `{}`", expected),
        ));
    }

    input.parse::<Token![:]>()?;

    Ok(())
}
