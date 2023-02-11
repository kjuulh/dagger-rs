use convert_case::{Case, Casing};
use dagger_core::introspection::{FullTypeFields, FullTypeFieldsArgs};
use genco::{prelude::rust, quote};

use super::{
    type_ref::{self, render_type_ref},
    utility::{render_description_from_field, render_description_from_input_value},
};

pub fn render_fields(
    fields: &Vec<FullTypeFields>,
) -> eyre::Result<Option<(rust::Tokens, rust::Tokens)>> {
    let mut collected_fields: Vec<rust::Tokens> = vec![];
    let mut collected_args: Vec<rust::Tokens> = vec![];
    for field in fields.iter() {
        let name = field.name.as_ref().map(|n| n.to_case(Case::Snake)).unwrap();
        let output = render_field_output(field)?;
        let description = render_description_from_field(field);
        let args = match field.args.as_ref() {
            Some(a) => render_args(a),
            None => None,
        };

        if let Some(args) = args.as_ref() {
            let mut args_tkns = rust::Tokens::new();
            args_tkns.append(quote! {
                $description
                pub struct $(&field.name.as_ref().map(|n| n.to_case(Case::Pascal)).unwrap())Args {
                    $(&args.args)
                }
            });
            args_tkns.push();

            collected_args.push(args_tkns);
        }
        let mut tkns = rust::Tokens::new();
        tkns.append(quote! {
            pub fn $(&name)(
                &self,
                $(if let Some(_) = args.as_ref() => args: &$(&field.name.as_ref().map(|n| n.to_case(Case::Pascal)).unwrap())Args)
            ) -> $(&output) {
                let query = self.selection.select($(field.name.as_ref().map(|n| format!("\"{}\"", n))));
                $(if let Some(_) = args.as_ref() => query.args(args);)

                $output {
                    conn: self.conn.clone(),
                    proc: self.proc.clone(),
                    selection: query,
                }

                todo!()
            }
        });

        collected_fields.push(tkns);
    }

    Ok(Some((
        quote! {
            $(for arg in collected_args => $arg $['\n'] )
        },
        quote! {
            $(for field in collected_fields => $field $['\n'] )
        },
    )))
}

struct Arg {
    name: String,
    description: Option<rust::Tokens>,
    type_: rust::Tokens,
}

struct CollectedArgs {
    description: Option<rust::Tokens>,
    args: rust::Tokens,
}

fn render_args(args: &[Option<FullTypeFieldsArgs>]) -> Option<CollectedArgs> {
    let mut collected_args: Vec<Arg> = vec![];

    for arg in args {
        if let Some(arg) = arg.as_ref().map(|a| &a.input_value) {
            let name = arg.name.clone();
            let description = render_description_from_input_value(&arg, &name);
            let t = render_type_ref(&arg.type_).unwrap();

            collected_args.push(Arg {
                name,
                description,
                type_: t,
            })
        }
    }

    if collected_args.len() > 0 {
        let mut collected_arg = CollectedArgs {
            description: Some(rust::Tokens::new()),
            args: rust::Tokens::new(),
        };

        for arg in collected_args {
            collected_arg.args.append(quote! {
                $(arg.description)
                pub $(arg.name.to_case(Case::Snake)): $(arg.type_),
            });
            collected_arg.args.push();
        }

        if let Some(desc) = collected_arg.description.as_ref() {
            if desc.is_empty() {
                collected_arg.description = None;
            }
        }

        Some(collected_arg)
    } else {
        None
    }
}

pub fn render_field_output(field: &FullTypeFields) -> eyre::Result<rust::Tokens> {
    let inner = &field.type_.as_ref().unwrap();
    type_ref::render_type_ref(&inner.type_ref)
}
