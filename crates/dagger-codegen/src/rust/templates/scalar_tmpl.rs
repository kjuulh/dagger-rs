use dagger_core::introspection::FullType;
use genco::prelude::rust;
use genco::quote;

use crate::rust::functions::format_name;
use crate::utility::OptionExt;

pub fn render_scalar(t: &FullType) -> eyre::Result<rust::Tokens> {
    Ok(quote! {
        pub struct $(t.name.pipe(|n|format_name(n)))(String);
    })
}
