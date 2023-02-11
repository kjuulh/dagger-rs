use dagger_core::introspection::FullType;
use genco::{prelude::rust, quote};

use crate::predicates::is_object_type;

use super::{fields, input_field, utility::render_description, Handler};

pub struct Object;

impl Handler for Object {
    fn predicate(&self, t: &FullType) -> bool {
        is_object_type(t)
    }

    fn render(&self, t: &FullType) -> eyre::Result<rust::Tokens> {
        let name = t
            .name
            .as_ref()
            .ok_or(eyre::anyhow!("could not find name"))?;
        let description = render_description(t);

        let fields = match t.fields.as_ref() {
            Some(i) => fields::render_fields(i)?,
            None => None,
        };

        let input_fields = match t.input_fields.as_ref() {
            Some(i) => input_field::render_input_fields(i)?,
            None => None,
        };

        let child = rust::import("std::process", "Child");
        let connect_params = rust::import("dagger_core::connect_params", "ConnectParams");
        let selection = rust::import("crate::querybuilder", "Selection");
        let arc = rust::import("std::sync", "Arc");

        let out = quote! {
            $(if fields.as_ref().is_some() => $(fields.as_ref().map(|f| &f.0)))

            $(if description.is_some() => $description)
            pub struct $name {
                $(if input_fields.is_some() => $input_fields)
                pub conn: $connect_params,
                pub proc: $arc<$child>,
                pub selection: $selection,
            }

            impl $name {
                $(if fields.is_some() => $(fields.map(|f| f.1)))
            }
        };

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use dagger_core::introspection::{
        FullType, FullTypeFields, FullTypeFieldsArgs, FullTypeFieldsType, InputValue, TypeRef,
        __TypeKind,
    };
    use pretty_assertions::assert_eq;

    use crate::handlers::Handler;

    use super::Object;

    #[test]
    fn can_render_object() {
        let t: FullType = FullType {
            kind: Some(__TypeKind::OBJECT),
            name: Some("CacheVolume".into()),
            description: Some("A directory whose contents persists across sessions".into()),
            fields: Some(vec![FullTypeFields {
                name: Some("id".into()),
                description: None,
                args: None,
                type_: Some(FullTypeFieldsType {
                    type_ref: TypeRef {
                        kind: Some(__TypeKind::NON_NULL),
                        name: None,
                        of_type: Some(Box::new(TypeRef {
                            kind: Some(__TypeKind::SCALAR),
                            name: Some("CacheID".into()),
                            of_type: None,
                        })),
                    },
                }),
                is_deprecated: Some(false),
                deprecation_reason: None,
            }]),
            input_fields: None,
            interfaces: None,
            enum_values: None,
            possible_types: None,
        };
        let expected = r#"use crate::querybuilder::Selection;
use dagger_core::connect_params::ConnectParams;
use std::process::Child;
use std::sync::Arc;


/// A directory whose contents persists across sessions
pub struct CacheVolume {
    pub conn: ConnectParams,
    pub proc: Arc<Child>,
    pub selection: Selection,
}

impl CacheVolume {
    pub fn id(
        &self,
    ) -> CacheID {
        let query = self.selection.select("id");

        CacheID {
            conn: self.conn.clone(),
            proc: self.proc.clone(),
            selection: query,
        }

        todo!()
    }
}
"#;
        let handler = Object {};
        let obj = handler.render(&t).unwrap();
        let actual = obj.to_file_string().unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn can_render_query_container() {
        let description = "Loads a container from ID.\nNull ID returns an empty container (scratch).\nOptional platform argument initializes new containers to execute and publish as that platform. Platform defaults to that of the builder's host.".into();

        let t: FullType = FullType {
            kind: Some(__TypeKind::OBJECT),
            name: Some("Query".into()),
            description: None,
            fields: Some(vec![FullTypeFields {
                name: Some("container".into()),
                description: Some(description),
                args: Some(vec![
                    Some(FullTypeFieldsArgs {
                        input_value: InputValue {
                            name: "id".into(),
                            description: None,
                            type_: TypeRef {
                                kind: Some(__TypeKind::SCALAR),
                                name: Some("ContainerID".into()),
                                of_type: None,
                            },
                            default_value: None,
                        },
                    }),
                    Some(FullTypeFieldsArgs {
                        input_value: InputValue {
                            name: "platform".into(),
                            description: None,
                            type_: TypeRef {
                                kind: Some(__TypeKind::SCALAR),
                                name: Some("Platform".into()),
                                of_type: None,
                            },
                            default_value: None,
                        },
                    }),
                ]),
                type_: Some(FullTypeFieldsType {
                    type_ref: TypeRef {
                        kind: Some(__TypeKind::NON_NULL),
                        name: None,
                        of_type: Some(Box::new(TypeRef {
                            kind: Some(__TypeKind::SCALAR),
                            name: Some("CacheID".into()),
                            of_type: None,
                        })),
                    },
                }),
                is_deprecated: Some(false),
                deprecation_reason: None,
            }]),
            input_fields: None,
            interfaces: None,
            enum_values: None,
            possible_types: None,
        };
        let expected = r#"use crate::querybuilder::Selection;
use dagger_core::connect_params::ConnectParams;
use std::process::Child;
use std::sync::Arc;


/// Loads a container from ID.
/// Null ID returns an empty container (scratch).
/// Optional platform argument initializes new containers to execute and publish as that platform. Platform defaults to that of the builder's host.
pub struct ContainerArgs {
    pub id: Option<ContainerID>,
    pub platform: Option<Platform>,
}

pub struct Query {
    pub conn: ConnectParams,
    pub proc: Arc<Child>,
    pub selection: Selection,
}

impl Query {
    pub fn container(
        &self,
        args: &ContainerArgs
    ) -> CacheID {
        let query = self.selection.select("container");
        query.args(args);

        CacheID {
            conn: self.conn.clone(),
            proc: self.proc.clone(),
            selection: query,
        }

        todo!()
    }
}
"#;
        let handler = Object {};
        let obj = handler.render(&t).unwrap();
        let actual = obj.to_file_string().unwrap();

        assert_eq!(actual, expected);
    }
}
