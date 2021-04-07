use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, TokenStreamExt};
use syn::{parse_macro_input, DeriveInput, Data, Fields};
use syn::spanned::Spanned;

#[proc_macro_derive(Decode)]
pub fn decode_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    
    let expr = gen_expr(&input.data);

    let expanded = quote! {
        impl Decode<#name> for #name {
            type Err = Error;

            fn decode(buf: &[u8]) -> Result<#name, Error> {
                let mut st = #name::default();

                for s in buf.split(|c| *c == b' ') {
                    let parts: Vec<&[u8]> = s.splitn(2, |c| *c == b'=').collect();

                    match *parts.get(0).unwrap() {
                        #expr
                        _ => (),
                    }
                }

                Ok(st)
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

fn gen_expr(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let recurse = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let ty = &f.ty;

                        let bytes = name.clone().unwrap().to_string().as_bytes().to_owned();
                        let bytes_fmt = bin_to_tokens(&bytes);

                        quote_spanned! {f.span()=> 
                            #bytes_fmt => {
                                st.#name = match <#ty>::decode(parts.get(1).unwrap()) {
                                Ok(val) => val,
                                Err(err) => return Err(err.into()),
                            }
                        },
                        }
                    });

                    quote! {
                        #(#recurse)*
                    }
                }
                _ => unimplemented!(),
            }
        }
        _ => unimplemented!(),
    }
}


fn bin_to_tokens(slice: &[u8]) -> TokenStream {
    let recurse = slice.iter().map(|b| {
        quote!(#b)
    });

    quote! {
        [#(#recurse),*]
    }
}
