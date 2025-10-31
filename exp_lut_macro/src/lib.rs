extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    Ident, LitFloat, LitInt, LitStr, Result, Token, parse::{Parse, ParseStream}, parse_macro_input
};

#[derive(Debug, Clone)]
struct ExpLutMacroInput {
    start: f32,
    end: f32,
    steps: usize,
}

impl Parse for ExpLutMacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let key1: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let lit1: LitFloat = input.parse()?;
        let value1: &str = lit1.base10_digits();

        input.parse::<Token![,]>()?;
        
        let key2: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let lit2: LitFloat = input.parse()?;
        let value2: &str = lit2.base10_digits();

        input.parse::<Token![,]>()?;

        let key3: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let lit3: LitInt = input.parse()?;
        let value3: &str = lit3.base10_digits();

        let mut input = ExpLutMacroInput {
            start: 0.0,
            end: 0.0,
            steps: 0,
        };
        
        for (key, value) in [(key1, value1), (key2, value2), (key3, value3)] {        
            if key == "start" {
                input.start = value.parse::<f32>().unwrap();
            } else if key == "end" {
                input.end = value.parse::<f32>().unwrap();
            } else if key == "steps" {
                input.steps = value.parse::<usize>().unwrap();
            }
        }

        Ok(input)
    }
}

#[proc_macro]
pub fn exp_lut_macro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ExpLutMacroInput);

    if input.steps < 2 {
        panic!("EXP_LOOKUP_STEPS must be >= 2");
    }

    if input.end <= input.start {
        panic!("EXP_LOOKUP_END must be > EXP_LOOKUP_START");
    }

    let steps: usize = input.steps;
    let start: f32 = input.start;
    let end: f32 = input.end;
    let step_size: f32 = (end - start) / steps as f32;

    let expanded = quote! {
        const LUT: [f32; #steps] = core::array::from_fn(|i| {
            let x = #start + (i as f32 * #step_size);
            x.exp()
        });
    };

    TokenStream::from(expanded)
}
