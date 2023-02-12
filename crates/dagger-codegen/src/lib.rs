mod functions;
mod generator;
pub mod rust;
mod visitor;

use dagger_core::introspection::Schema;

use self::generator::DynGenerator;

pub fn generate(schema: Schema, generator: DynGenerator) -> eyre::Result<String> {
    generator.generate(schema)
}
