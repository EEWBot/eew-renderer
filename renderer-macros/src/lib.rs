use proc_macro::TokenStream;
use syn::{parse_macro_input, LitStr, LitFloat, Token};
use syn::parse::{Parse, ParseStream};
use quote::quote;

struct RgbArgs {
    hex: LitStr,
}

impl Parse for RgbArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(RgbArgs {
            hex: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn rgb(input: TokenStream) -> TokenStream {
    let RgbArgs { hex, .. } = parse_macro_input!(input as RgbArgs);

    let hex_str = hex.value();
    let hex_str = hex_str.trim_start_matches('#');
    let r = u8::from_str_radix(&hex_str[0..2], 16).unwrap();
    let g = u8::from_str_radix(&hex_str[2..4], 16).unwrap();
    let b = u8::from_str_radix(&hex_str[4..6], 16).unwrap();

    let expanded = quote! {
        [
            #r as f32 / 255.0,
            #g as f32 / 255.0,
            #b as f32 / 255.0,
        ]
    };

    expanded.into()
}

struct RgbaArgs {
    hex: LitStr,
    _comma: Token![,],
    alpha: LitFloat,
}

impl Parse for RgbaArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(RgbaArgs {
            hex: input.parse()?,
            _comma: input.parse()?,
            alpha: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn rgba(input: TokenStream) -> TokenStream {
    let RgbaArgs { hex, alpha, .. } = parse_macro_input!(input as RgbaArgs);

    let hex_str = hex.value();
    let hex_str = hex_str.trim_start_matches('#');
    let r = u8::from_str_radix(&hex_str[0..2], 16).unwrap();
    let g = u8::from_str_radix(&hex_str[2..4], 16).unwrap();
    let b = u8::from_str_radix(&hex_str[4..6], 16).unwrap();

    let expanded = quote! {
        [
            #r as f32 / 255.0,
            #g as f32 / 255.0,
            #b as f32 / 255.0,
            #alpha,
        ]
    };

    expanded.into()
}
