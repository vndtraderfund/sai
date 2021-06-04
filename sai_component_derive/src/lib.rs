//! This crate provides Sai's component derive macros:
//!
//! ```ignore
//! use sai::{Component};
//! #[derive(Component)]
//! struct FooComponent {
//!     #[injected]
//!     bar: Injected<BarComponent>
//! }
//! ```
//!
//! Please refer the Component Trait doc to understand details.

/// Outer What is this?

extern crate proc_macro;
use proc_macro::TokenStream;
#[macro_use]
extern crate syn;
use syn::{DeriveInput, Data, Field};
use syn::Meta::{Path};
use proc_macro2::TokenStream as TokenStream2;
#[macro_use]
extern crate quote;

mod symbol;
use symbol::*;


/// Dervive Macro for defining a component
#[proc_macro_derive(Component, attributes(injected, lifecycle))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    // println!("{}", ast.attrs);
    let ident = &ast.ident;

    let fields = parse_struct_data(&ast.data);

    let fields_tokens = build_struct_fields(&fields);

    let depends_on_tokens = build_depends_on(&fields);

    let impl_lifecycle_tokens = build_impl_component_lifecycle(&ast);

    let tokens = quote!{
        impl sai::Component for #ident {
            fn build(registry: &sai::ComponentRepository) -> #ident {
                return #ident {
                    #fields_tokens
                }

            }

            // Used during topology sort to calculate DAG
            fn meta() -> sai::ComponentMeta<std::boxed::Box<#ident>> {

                return sai::ComponentMeta {
                    type_id: std::any::TypeId::of::<sai::Injected<#ident>>(),
                    depends_on: #depends_on_tokens,
                    build: std::boxed::Box::new(
                        |repo: &sai::ComponentRepository| std::boxed::Box::new(#ident::build(repo))
                    )
                }
            }
        }

        #impl_lifecycle_tokens
    };

    return tokens.into();
}

fn build_struct_fields (fields: &Vec<ComponentField>) -> TokenStream2 {

    let x: Vec<TokenStream2> = fields
        .into_iter()
        .map(|f| {
            let ident = &f.ident;
            let ty = &f.ty; // expecting Injected<Bluh>
            if f.injected {
                let error_msg_type_not_found = format!("Unable to find type {} in component repository", quote!{#ty});
                let error_msg_cast_failure = format!("Found {} in component repository. But unable to downcast it", quote!{#ty});

                return quote! {
                    #ident: {
                        let comp: &sai::Injected::<dyn sai::Component> = registry.get_by_typeid(std::any::TypeId::of::<#ty>()).expect(#error_msg_type_not_found);
                        let dep: #ty = comp.clone().downcast().expect(#error_msg_cast_failure);
                        dep
                    }
                }
            } else {
                return quote! {
                    #ident: Default::default()
                }
            }
        })
        .collect();


    quote!{
        #(#x),*
    }
}

fn build_depends_on(fields: &Vec<ComponentField>) -> TokenStream2 {

    let x: Vec<TokenStream2> = fields
        .into_iter()
        .filter(|f| f.injected)
        .map(|f| {
            let ty = &f.ty;
            return quote! {
                std::any::TypeId::of::<#ty>()
            }
        })
        .collect();

    quote! {
        vec![ #(#x),* ]
    }
}

fn build_impl_component_lifecycle(ast: &DeriveInput) -> TokenStream2 {
    let ident = &ast.ident;

    if is_lifecycle_mode(ast) {
        return quote! {}
    } else {
        return quote! {
            impl sai::ComponentLifecycle for #ident {}
        }
    }
}

fn is_lifecycle_mode(ast: &DeriveInput) -> bool {
    return ast
        .attrs
        .iter()
        .map(|attr| attr.path == LIFECYCLE)
        .any(|i| i);
}


struct ComponentField {
    injected: bool,
    ident: syn::Ident,
    ty: syn::Type
}

fn parse_struct_data (data: &Data) -> Vec<ComponentField> {

    let s = match data {
        Data::Struct(s) => s,
        _ => panic!("Component macro can only be used on struct enum")
    };


    let fields = match &s.fields {
        syn::Fields::Named(f) => f,
        syn::Fields::Unit => return Vec::new(),
        _ => panic!("Component marco can not be used on tuple struct")
    };
    let fields = &fields.named;

    return fields
        .iter()
        .map(parse_struct_field)
        .collect::<Vec<ComponentField>>();
}

fn parse_struct_field (field: &Field) -> ComponentField {

    let ty = field.ty.clone();
    let ident = field.ident.clone().unwrap();
    let attrs = &field.attrs;

    let injected = attrs
        .iter()
        .map(|attr| is_injected_attribute(attr))
        .any(|i| match i {
            Ok(v) => v,
            Err(_) => false // TODO: improve error handling
        });


    return ComponentField {
        injected,
        ident,
        ty
    }
}

fn is_injected_attribute(attr: &syn::Attribute) -> Result<bool, ()> {
    if attr.path != INJECTED {
        return Ok(false)
    }

    match attr.parse_meta() {
        Ok(Path(_)) => Ok(true), // Only expect #[injected]
        Ok(_) => Err(()), // TODO: improve error handling
        Err(_) => Err(()) // TODO: improve error handling
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

