//! Tools for compiling and linking SPIR-V shaders.
//! To use this module, the [Vulkan SDK](https://www.lunarg.com/vulkan-sdk/) (specifically
//! `glslangValidator` and `spirv-link`) should be added to your `PATH` if not already present.
#![cfg(feature = "spirv")]
#![allow(dead_code)]

use rand::{distributions, Rng};
use std::ffi::{OsStr, OsString};
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{error, fmt, io};

/// The Vulkan target used with the Vulkan SDK.
const VULKAN_TARGET: &'static str = "vulkan1.3";

/// Represents a SPIR-V shader stage.
/// # WGPU Support
/// WGPU doesn't support geometry & tesselation shaders. They're only listed here for completeness.
#[derive(Clone, Copy, Debug)]
pub enum Stage {
    Vertex,
    Fragment,
    TesselationControl,
    TesselationEvaluation,
    Geometry,
    Compute,
}
impl Stage {
    /// Returns an identifier for the current shader stage. This identifier is recognizable by
    /// `glslangValidator` and `glslc`.
    fn identifier(&self) -> &'static str {
        match self {
            Self::Vertex => "vert",
            Self::Fragment => "frag",
            Self::TesselationControl => "tesc",
            Self::TesselationEvaluation => "tese",
            Self::Geometry => "geom",
            Self::Compute => "comp",
        }
    }
}

/// Represents an error which occured in the SPIR-V build process.
#[derive(Debug)]
pub enum BuildError {
    /// An error occured while compiling a shader source to a SPIR-V binary.
    /// The wrapped `String` should explain the error, while the wrapped `PathBuf` contains a
    /// path to the shader source which caused the error.
    CompileError(String, PathBuf),
    /// An error occured when linking SPIR-V binaries. The wrapped `String` should explain the
    /// error.
    LinkError(String),
    /// The wrapped I/O error occured outside of the compiler/linker.
    IoError(io::Error),
}
impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CompileError(compile_error, path) => {
                let path_display = path.display();
                write!(f, "{path_display}: SPIR-V compile failure: {compile_error}")
            }
            Self::LinkError(link_error) => write!(f, "SPIR-V link failure: {link_error}"),
            Self::IoError(io_error) => write!(f, "SPIR-V build I/O failure: {io_error}"),
        }
    }
}
impl error::Error for BuildError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            _ => None,
        }
    }
}

/// Represents a unique, randomized temporary path. When a temporary path is `Drop`-ed, it
/// attempts to remove the file at that path.
struct TemporaryPath(PathBuf);
impl TemporaryPath {
    /// Creates a new temporary path. The path will start with `prefix`. The rest of the path is
    /// randomized.
    fn new(prefix: impl AsRef<OsStr>) -> Self {
        let temp_dir = std::env::temp_dir();
        let mut filename = prefix.as_ref().to_os_string();
        filename.push(".");
        let suffix = String::from_iter(
            rand::thread_rng()
                .sample_iter(distributions::Alphanumeric)
                .take(16),
        );
        filename.push(OsString::from(suffix));
        Self(temp_dir.join(filename))
    }
    /// Return a reference to the path wrapped by this instance.
    fn as_path(&self) -> &Path {
        self.0.as_path()
    }
}
impl AsRef<Path> for TemporaryPath {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}
impl AsRef<OsStr> for TemporaryPath {
    fn as_ref(&self) -> &OsStr {
        self.0.as_ref()
    }
}
impl Drop for TemporaryPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.as_path());
    }
}

/// Represents a shader source file's type (that is, whether it's GLSL or HLSL) and contains
/// any GLSL or HLSL-specific information which is required to compile that shader.
/// A precompiled SPIR-V binary is considerded a shader, but not a shader source.
pub enum SourceType<'a> {
    Glsl {
        /// Every GLSL shader's entry point must be named `"main"`. This is rather inflexible and
        /// inevitably leads to naming conflicts if you link multiple compiled GLSL shaders into a
        /// single SPIR-V binary. As a workaround, the entry point of the compiled shader will be
        /// renamed from `"main"` to `output_entry_point` in the compiled shader binary.
        output_entry_point: &'a str,
    },
    Hlsl {
        /// The entry point of the HLSL shader.
        entry_point: &'a str,
    },
}

/// Compiles the GLSL or HLSL shader at `path` to SPIR-V, (over)writing the output at
/// `output_path`. The source type (and any type-specific information) is specified by
/// `source_type`, and the shader stage is specified by `stage`.
pub fn compile_from(
    path: &Path,
    source_type: SourceType,
    stage: Stage,
    output_path: &Path,
) -> Result<(), BuildError> {
    // Build basic command (shared between HLSL and GLSL)
    let mut command = Command::new("glslangValidator");
    command
        .arg("--quiet")
        .args(&["--target-env", VULKAN_TARGET])
        .args(&["-S", stage.identifier()])
        .args(&[OsStr::new("-o"), output_path.as_os_str()]);
    // Add GLSL-specific or HLSL-specific arguments
    match source_type {
        SourceType::Glsl { output_entry_point } => {
            command.arg("--enhanced-msgs"); // fun GLSL-only bonus
            command.args(&["--source-entrypoint", "main"]);
            command.args(&["--entry-point", output_entry_point]);
        }
        SourceType::Hlsl { entry_point } => {
            command.arg("-D"); // informs compiler that this is HLSL
            command.args(&["--entry-point", entry_point]);
        }
    }
    // Add the actual source path to the shader
    command.arg(path);
    // Compile & check output
    match command.output() {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let description =
                format!("[glslangValidator stdout]: {stdout}\n[glslangValidator stderr]: {stderr}");
            Err(BuildError::CompileError(description, path.to_path_buf()))
        }
        Err(io) => Err(BuildError::IoError(io)),
    }
}

/// Links together SPIR-V binaries with paths from `spirv_paths` into a single SPIR-V binary
/// and (over)writes that binary to `output_path`.
pub fn link<'a>(
    spirv_paths: impl Iterator<Item = &'a Path>,
    output_path: &Path,
) -> Result<(), BuildError> {
    // Setup command, --verify-ids isn't strictly necessary but why not?
    let mut command = Command::new("spirv-link");
    command.arg("--verify-ids");
    command.args(&["--target-env", VULKAN_TARGET]);
    command.args(&[OsStr::new("-o"), output_path.as_os_str()]);
    // Add each SPIR-V binary path
    for spirv_path in spirv_paths {
        command.arg(spirv_path);
    }
    // Compile & check output
    match command.output() {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let description =
                format!("[spirv-link stdout]: {stdout}\n[spirv-link stderr]: {stderr}");
            Err(BuildError::LinkError(description))
        }
        Err(io) => Err(BuildError::IoError(io)),
    }
}

/// Represents a component of a SPIR-V module. A component consists of a shader, which is either
/// precompiled into SPIR-V or a GLSL/HLSL source file. If it's a source file, the enum variant
/// contains any information which is necessary to compile that component.
pub enum Component<'a, P: 'a + AsRef<Path>> {
    SpirV {
        /// A path to the compiled SPIR-V binary.
        path: P,
    },
    Glsl {
        /// A path to the GLSL shader.
        path: P,
        /// The shader stage of the GLSL shader.
        stage: Stage,
        /// See the `output_entry_point` field of [`SourceType::Glsl`].
        output_entry_point: &'a str,
    },
    Hlsl {
        /// A path to the HLSL shader.
        path: P,
        /// The shader stage of the HLSL shader — or perhaps more accurately, the shader
        /// stage of `entry_point`, since a single HLSL source file can contain multiple shaders.
        stage: Stage,
        /// The entry point of the HLSL shader.
        entry_point: &'a str,
    },
}
impl<'a, P: 'a + AsRef<Path>> Component<'a, P> {
    /// Returns the [`SourceType`] which best represents this component.
    /// # Panics
    /// Panics if this component is `SpirV`. SPIR-V is not a source format and has no source type.
    fn source_type(&self) -> SourceType<'a> {
        match self {
            Self::SpirV { .. } => panic!("source_type called on SPIR-V component"),
            Self::Glsl {
                output_entry_point, ..
            } => SourceType::Glsl { output_entry_point },
            Self::Hlsl { entry_point, .. } => SourceType::Hlsl { entry_point },
        }
    }
}

/// Convinence function to quickly compile/link one or more components into a SPIR-V module.
/// Upon success, returns that binary as a byte vector.
/// If more control is desired, consider using [`compile_from`] and [`link`] instead.
pub fn build<'a, P: 'a + AsRef<Path>>(
    components: &[Component<'a, P>],
) -> Result<Vec<u8>, BuildError> {
    let mut temporary_spirvs = Vec::new();
    let mut explicit_spirvs = Vec::new();
    for component in components {
        match component {
            Component::Glsl { path, stage, .. } | Component::Hlsl { path, stage, .. } => {
                let temp_path = TemporaryPath::new(path.as_ref().file_name().unwrap_or_default());
                compile_from(
                    path.as_ref(),
                    component.source_type(),
                    *stage,
                    temp_path.as_path(),
                )?;
                temporary_spirvs.push(temp_path);
            }
            Component::SpirV { path } => explicit_spirvs.push(path.as_ref()),
        }
    }
    let spirv_paths = temporary_spirvs
        .iter()
        .map(|temp| temp.as_path())
        .chain(explicit_spirvs.into_iter());
    let linked_path = TemporaryPath::new("linked");
    link(spirv_paths, linked_path.as_path())?;
    match std::fs::read(linked_path.as_path()) {
        Ok(bytes) => Ok(bytes),
        Err(io_error) => Err(BuildError::IoError(io_error)),
    }
}

// Since this is in the examples directory, cargo thinks this is an example, so it looks for
// a main function. This really doesn't belong in the examples directory, but it feels weird
// making a seperate crate for this, and `framework.rs` does this too.
fn main() {}
