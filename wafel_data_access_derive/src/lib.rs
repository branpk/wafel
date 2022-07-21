//! Derive macro for wafel_data_access::DataReadable.

use heck::ToLowerCamelCase;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, FieldsNamed};

#[proc_macro_derive(
    DataReadable,
    attributes(struct_name, struct_anon, field_offset, field_name)
)]
pub fn derive_data_readable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let reader_name = Ident::new(&format!("{}Reader", input.ident), input.ident.span());

    let reader = generate_reader(&input, &reader_name);
    let reader_impl = generate_reader_impl(&input, &reader_name);
    let readable_impl = generate_readable_impl(&input, &reader_name);

    quote! {
        #reader
        #reader_impl
        #readable_impl
    }
    .into()
}

fn generate_reader(input: &DeriveInput, reader_name: &Ident) -> TokenStream {
    let vis = &input.vis;
    let fields = named_fields(input);

    let reader_fields = fields.named.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;

        let name = Ident::new(&format!("field_{}", field_name), field_name.span());
        quote! {
            #name: (usize, wafel_data_access::Reader<#field_ty>),
        }
    });

    quote! {
        #vis struct #reader_name {
            #(#reader_fields)*
        }
    }
}

fn generate_reader_impl(input: &DeriveInput, reader_name: &Ident) -> TokenStream {
    let vis = &input.vis;
    let name = &input.ident;
    let fields = named_fields(input);

    let field_inits = fields.named.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let name = Ident::new(&format!("field_{}", field_name), field_name.span());

        quote! {
            #field_name: self.#name.1.read(memory, addr + self.#name.0)?,
        }
    });

    quote! {
        #[allow(unused_parens)]
        impl #reader_name {
            #vis fn read(
                &self,
                memory: &impl wafel_memory::MemoryRead,
                addr: wafel_data_type::Address,
            ) -> Result<#name, wafel_data_access::DataError> {
                Ok(#name {
                    #(#field_inits)*
                })
            }
        }

        impl wafel_data_access::DataReader for #reader_name {
            type Output = #name;

            fn read(
                &self,
                memory: &impl wafel_memory::MemoryRead,
                addr: wafel_data_type::Address,
            ) -> Result<#name, wafel_data_access::DataError> {
                self.read(memory, addr)
            }
        }
    }
}

fn generate_readable_impl(input: &DeriveInput, reader_name: &Ident) -> TokenStream {
    let name = &input.ident;
    let fields = named_fields(input);

    let struct_name = name.to_string();
    let mut used_struct_name = Some(quote! { #struct_name });
    for attr in &input.attrs {
        if attr.path.is_ident("struct_name") {
            used_struct_name = Some(attr.tokens.clone());
        } else if attr.path.is_ident("struct_anon") {
            used_struct_name = None;
        }
    }

    let field_inits = fields.named.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;
        let name = Ident::new(&format!("field_{}", field_name), field_name.span());

        let camel_case_field_name = field_name.to_string().to_lower_camel_case();
        let mut used_field_name = quote! { #camel_case_field_name };

        let mut offset: Option<TokenStream> = None;
        for attr in &field.attrs {
            if attr.path.is_ident("field_offset") {
                offset = Some(attr.tokens.clone());
            } else if attr.path.is_ident("field_name") {
                used_field_name = attr.tokens.clone();
            }
        }

        if offset.is_none() && used_struct_name.is_none() {
            panic!("anonymous struct requires explicit #[field_offset(..)]");
        }

        let offset = offset.unwrap_or_else(|| {
            quote! {
                data_type.struct_field(#used_field_name)?.offset
            }
        });

        quote! {
            #name: (
                #offset,
                <#field_ty as wafel_data_access::DataReadable>::reader(layout)?,
            ),
        }
    });

    let calc_data_type = used_struct_name.as_ref().map(|name| {
        quote! {
            let type_name = wafel_data_type::TypeName::of_struct(#name);
            let data_type = layout.data_layout().data_type(&type_name)?;
        }
    });

    quote! {
        #[allow(unused_parens)]
        impl wafel_data_access::DataReadable for #name {
            type Reader = #reader_name;

            fn reader(
                layout: &impl wafel_data_access::MemoryLayout
            ) -> Result<#reader_name, wafel_data_access::DataError> {
                #calc_data_type
                Ok(#reader_name {
                    #(#field_inits)*
                })
            }
        }
    }
}

fn named_fields(input: &DeriveInput) -> &FieldsNamed {
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields,
            Fields::Unnamed(_) => unimplemented!("derive DataReadable on tuple struct"),
            Fields::Unit => unimplemented!("derive DataReadable on unit struct"),
        },
        Data::Enum(_) => unimplemented!("derive DataReadable on enum"),
        Data::Union(_) => unimplemented!("derive DataReadable on union"),
    }
}
