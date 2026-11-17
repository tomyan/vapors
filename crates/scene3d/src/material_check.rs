//! A material's WGSL checked when it is given (`Meshes::material`):
//! parsed and validated with [`MATERIAL_WGSL`] and nothing else, so it
//! can use only the contract. A reference to the scene renderer's
//! `camera` or `node`, or to anything else outside it, does not resolve;
//! a binding of its own, a name the renderers use beside it
//! ([`RESERVED_NAMES`]), or a `material` function of another shape is
//! refused. Every refusal names the material's label and, where it has
//! one, a line counted in the material's own source.

use std::fmt;

use naga::{Module, Scalar, TypeInner, VectorSize};

use crate::{MATERIAL_WGSL, Material};

/// The names the renderers define beside a material, which it must not
/// define: `vapors_renderer`'s `PROJECTIVE_WGSL` and `vapors-scene3d-gpu`'s
/// `material.wgsl`, held to them by `scene3d-gpu`'s
/// `pipeline_tests.rs`.
pub const RESERVED_NAMES: &[&str] = &[
    "Camera",
    "MaterialVertex",
    "NEAR_W",
    "Node",
    "camera",
    "edge_on",
    "fs_material",
    "jacobian_at",
    "node",
    "on_plane",
    "screen_jacobian",
    "shape_to_clip",
    "to_clip",
    "vs_material",
];

/// Why a material was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterialError {
    /// Its WGSL does not parse or validate against the contract: the
    /// line, counted in its own source, where there is one.
    Wgsl {
        /// The material's label.
        label: &'static str,
        /// The line in its own source, 1-based.
        line: Option<u32>,
        /// naga's message.
        message: String,
    },
    /// It declares a resource binding: a material reads only what the
    /// contract gives it.
    Binding {
        /// The material's label.
        label: &'static str,
        /// The binding's name.
        name: String,
    },
    /// It defines a name the renderers use beside it.
    Reserved {
        /// The material's label.
        label: &'static str,
        /// The name.
        name: String,
    },
    /// It has no `fn material(at: MaterialInput) -> vec4<f32>`.
    Signature {
        /// The material's label.
        label: &'static str,
    },
}

impl fmt::Display for MaterialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wgsl {
                label,
                line: Some(line),
                message,
            } => {
                write!(
                    f,
                    "material `{label}`, line {line}: {message}; a material reads only MaterialInput (vapors-scene3d MATERIAL_WGSL)"
                )
            }
            Self::Wgsl {
                label,
                line: None,
                message,
            } => {
                write!(
                    f,
                    "material `{label}`: {message}; a material reads only MaterialInput (vapors-scene3d MATERIAL_WGSL)"
                )
            }
            Self::Binding { label, name } => write!(
                f,
                "material `{label}` binds `{name}`: a material reads only MaterialInput; pass what it needs as constants in its source"
            ),
            Self::Reserved { label, name } => write!(
                f,
                "material `{label}` defines `{name}`, which the renderer defines beside it: rename it"
            ),
            Self::Signature { label } => write!(
                f,
                "material `{label}` has no `fn material(at: MaterialInput) -> vec4<f32>`: define it"
            ),
        }
    }
}

impl std::error::Error for MaterialError {}

/// `material`, refused as [`MaterialError`] says.
pub(crate) fn check(material: &Material) -> Result<(), MaterialError> {
    let source = [MATERIAL_WGSL, material.wgsl.as_str()].concat();
    let module = parsed(material, &source)?;
    no_bindings(material, &module)?;
    no_reserved(material, &module)?;
    has_material_fn(material, &module)
}

/// The module of `source`, the contract then the material, parsed and
/// validated.
fn parsed(material: &Material, source: &str) -> Result<Module, MaterialError> {
    let refused = |line: Option<u32>, message: String| MaterialError::Wgsl {
        label: material.label,
        line: line.and_then(own_line),
        message,
    };
    let module = naga::front::wgsl::parse_str(source).map_err(|error| {
        let line = error.location(source).map(|at| at.line_number);
        refused(line, error.message().to_owned())
    })?;
    let flags = naga::valid::ValidationFlags::all();
    let mut validator = naga::valid::Validator::new(flags, naga::valid::Capabilities::all());
    validator.validate(&module).map_err(|error| {
        let line = error.location(source).map(|at| at.line_number);
        refused(line, error.as_inner().to_string())
    })?;
    Ok(module)
}

/// `line` of the joined source as a line of the material's own: `None`
/// in the contract.
fn own_line(line: u32) -> Option<u32> {
    let contract = u32::try_from(MATERIAL_WGSL.matches('\n').count()).ok()?;
    line.checked_sub(contract)
}

fn no_bindings(material: &Material, module: &Module) -> Result<(), MaterialError> {
    let bound = module.global_variables.iter().map(|(_, global)| global);
    match bound.into_iter().find(|global| global.binding.is_some()) {
        Some(global) => Err(MaterialError::Binding {
            label: material.label,
            name: global.name.clone().unwrap_or_default(),
        }),
        None => Ok(()),
    }
}

fn no_reserved(material: &Material, module: &Module) -> Result<(), MaterialError> {
    let functions = module
        .functions
        .iter()
        .filter_map(|(_, f)| f.name.as_deref());
    let types = module.types.iter().filter_map(|(_, t)| t.name.as_deref());
    let constants = module
        .constants
        .iter()
        .filter_map(|(_, c)| c.name.as_deref());
    let globals = module
        .global_variables
        .iter()
        .filter_map(|(_, g)| g.name.as_deref());
    let entries = module.entry_points.iter().map(|entry| entry.name.as_str());
    let mut names = functions
        .chain(entries)
        .chain(types)
        .chain(constants)
        .chain(globals);
    match names.find(|name| RESERVED_NAMES.contains(name)) {
        Some(name) => Err(MaterialError::Reserved {
            label: material.label,
            name: name.to_owned(),
        }),
        None => Ok(()),
    }
}

/// `fn material(at: MaterialInput) -> vec4<f32>`, or the refusal.
fn has_material_fn(material: &Material, module: &Module) -> Result<(), MaterialError> {
    let found = module.functions.iter().map(|(_, f)| f);
    let shaped = found
        .into_iter()
        .find(|f| f.name.as_deref() == Some("material"))
        .is_some_and(|f| {
            let input = f.arguments.len() == 1
                && module.types[f.arguments[0].ty].name.as_deref() == Some("MaterialInput");
            let output = f.result.as_ref().is_some_and(|result| {
                module.types[result.ty].inner
                    == TypeInner::Vector {
                        size: VectorSize::Quad,
                        scalar: Scalar::F32,
                    }
            });
            input && output
        });
    if shaped {
        return Ok(());
    }
    Err(MaterialError::Signature {
        label: material.label,
    })
}

#[cfg(test)]
#[path = "material_check_tests.rs"]
mod tests;
