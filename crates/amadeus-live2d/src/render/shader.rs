//! GL programs: compile/link (`loadShaderProgram` / `compileShaderSource`), the per-program
//! locations (`CubismShaderSet`) and the program cache (render spec §7.3–7.4, DECISIONS D4).
//!
//! V2 differences (DEVIATION-OK, render spec §7.3 recommendation): the sources are embedded and
//! compiled synchronously; only the four pre-5.3 programs exist and each is compiled the first
//! time it is needed (sets 4–9 reuse 1–3); a program that fails to compile/link is logged once
//! and marked failed (no per-frame retry, no program leak) — the renderer then falls back so the
//! model never becomes invisible because of an unused program. The 237 Cubism 5.3 blend
//! programs and the copy program are never compiled (unsupported, see `renderer`).

use std::borrow::Cow;

use wasm_bindgen::JsValue;
use web_sys::{WebGlProgram, WebGlShader, WebGlUniformLocation};

use super::gl::{Gl, GlConst as GL};
use super::shader_sources as src;
use super::shader_table::BaseProgram;
use super::RenderError;

/// `CubismShaderSet`: a program and the locations V1 queries for it (a location V1 never
/// queries stays `None`; uniform calls with `None` are no-ops like `uniform*(undefined, …)`).
#[derive(Debug)]
pub struct ShaderSet {
    pub program: WebGlProgram,
    pub a_position: i32,
    pub a_tex_coord: i32,
    pub u_matrix: Option<WebGlUniformLocation>,
    pub u_clip_matrix: Option<WebGlUniformLocation>,
    pub s_texture0: Option<WebGlUniformLocation>,
    pub s_texture1: Option<WebGlUniformLocation>,
    pub u_base_color: Option<WebGlUniformLocation>,
    pub u_channel_flag: Option<WebGlUniformLocation>,
    pub u_multiply_color: Option<WebGlUniformLocation>,
    pub u_screen_color: Option<WebGlUniformLocation>,
}

impl ShaderSet {
    /// Queries the locations `registerShader` queries for the given program.
    fn query(gl: &Gl, program: WebGlProgram, which: BaseProgram) -> Self {
        let uniform = |name: &str| gl.get_uniform_location(&program, name);
        let (u_matrix, u_clip_matrix, s_texture1, u_channel_flag, u_multiply_color, u_screen_color) =
            match which {
                BaseProgram::SetupMask => (
                    None,
                    uniform("u_clipMatrix"),
                    None,
                    uniform("u_channelFlag"),
                    None,
                    None,
                ),
                BaseProgram::Normal => (
                    uniform("u_matrix"),
                    None,
                    None,
                    None,
                    uniform("u_multiplyColor"),
                    uniform("u_screenColor"),
                ),
                BaseProgram::Masked | BaseProgram::MaskedInverted => (
                    uniform("u_matrix"),
                    uniform("u_clipMatrix"),
                    uniform("s_texture1"),
                    uniform("u_channelFlag"),
                    uniform("u_multiplyColor"),
                    uniform("u_screenColor"),
                ),
            };
        let s_texture0 = uniform("s_texture0");
        let u_base_color = uniform("u_baseColor");
        Self {
            a_position: gl.get_attrib_location(&program, "a_position"),
            a_tex_coord: gl.get_attrib_location(&program, "a_texCoord"),
            u_matrix,
            u_clip_matrix,
            s_texture0,
            s_texture1,
            u_base_color,
            u_channel_flag,
            u_multiply_color,
            u_screen_color,
            program,
        }
    }
}

/// `(vertex, fragment)` sources of a base program.
pub fn base_program_sources(which: BaseProgram) -> (&'static str, &'static str) {
    match which {
        BaseProgram::SetupMask => (
            src::VERT_SHADER_SRC_SETUP_MASK,
            src::FRAG_SHADER_SRC_SETUP_MASK,
        ),
        BaseProgram::Normal => (
            src::VERT_SHADER_SRC,
            src::FRAG_SHADER_SRC_PREMULTIPLIED_ALPHA,
        ),
        BaseProgram::Masked => (
            src::VERT_SHADER_SRC_MASKED,
            src::FRAG_SHADER_SRC_MASK_PREMULTIPLIED_ALPHA,
        ),
        BaseProgram::MaskedInverted => (
            src::VERT_SHADER_SRC_MASKED,
            src::FRAG_SHADER_SRC_MASK_INVERTED_PREMULTIPLIED_ALPHA,
        ),
    }
}

/// `compileShaderSource(type, src)`: logs `Cubism shader compile failed.` (V1 console string) on
/// failure and deletes the shader.
fn compile_shader_source(
    gl: &Gl,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, RenderError> {
    let stage = if shader_type == GL::VERTEX_SHADER {
        "vertex"
    } else {
        "fragment"
    };
    let Some(shader) = gl.create_shader(shader_type) else {
        return Err(RenderError::ShaderCompile {
            stage,
            log: "createShader returned null (context lost?)".to_owned(),
        });
    };
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    if !gl
        .get_shader_parameter(&shader, GL::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        if gl.is_context_lost() {
            gl.delete_shader(Some(&shader));
            return Err(RenderError::ContextLost);
        }
        let log = gl.get_shader_info_log(&shader).unwrap_or_default();
        web_sys::console::error_2(
            &JsValue::from_str("Cubism shader compile failed."),
            &JsValue::from_str(&log),
        );
        log::error!(target: "cubism", "Shader compile log: {log} ");
        gl.delete_shader(Some(&shader));
        return Err(RenderError::ShaderCompile { stage, log });
    }
    Ok(shader)
}

/// `loadShaderProgram(vs, fs)`: compiles, links, deletes the shader objects. Logs the V1 console
/// strings on failure (`Cubism shader compile failed.` / `Cubism shader link failed.`) and, unlike
/// the SDK, deletes every object created on the failure paths.
pub fn compile_program(
    gl: &Gl,
    vertex_source: &str,
    fragment_source: &str,
) -> Result<WebGlProgram, RenderError> {
    let program = gl.create_program().ok_or(RenderError::ContextLost)?;
    let vertex = match compile_shader_source(gl, GL::VERTEX_SHADER, vertex_source) {
        Ok(s) => s,
        Err(e) => {
            log::error!(target: "cubism", "Vertex shader compile error!");
            gl.delete_program(Some(&program));
            return Err(e);
        }
    };
    let fragment = match compile_shader_source(gl, GL::FRAGMENT_SHADER, fragment_source) {
        Ok(s) => s,
        Err(e) => {
            log::error!(target: "cubism", "Fragment shader compile error!");
            gl.delete_shader(Some(&vertex));
            gl.delete_program(Some(&program));
            return Err(e);
        }
    };
    gl.attach_shader(&program, &vertex);
    gl.attach_shader(&program, &fragment);
    gl.link_program(&program);
    if !gl
        .get_program_parameter(&program, GL::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        let log = gl.get_program_info_log(&program).unwrap_or_default();
        web_sys::console::error_2(
            &JsValue::from_str("Cubism shader link failed."),
            &JsValue::from_str(&log),
        );
        log::error!(target: "cubism", "Failed to link program: {log}");
        gl.delete_shader(Some(&vertex));
        gl.delete_shader(Some(&fragment));
        gl.delete_program(Some(&program));
        return Err(RenderError::ShaderLink { log });
    }
    gl.delete_shader(Some(&vertex));
    gl.delete_shader(Some(&fragment));
    Ok(program)
}

/// Upgrades `precision mediump float;` in embedded Cubism fragment shaders to `highp` when
/// supported (`GL_FRAGMENT_PRECISION_HIGH`) so UV coordinates stay accurate on 8192×8192 atlases.
fn upgrade_fragment_precision(source: &str) -> Cow<'_, str> {
    const TARGET: &str = "precision mediump float;";
    const REPLACEMENT: &str =
        "#ifdef GL_FRAGMENT_PRECISION_HIGH\nprecision highp float;\n#else\nprecision mediump float;\n#endif";
    let Some(pos) = source.find(TARGET) else {
        return Cow::Borrowed(source);
    };
    let mut upgraded = String::with_capacity(source.len() + REPLACEMENT.len());
    upgraded.push_str(&source[..pos]);
    upgraded.push_str(REPLACEMENT);
    upgraded.push_str(&source[pos + TARGET.len()..]);
    Cow::Owned(upgraded)
}

#[derive(Debug, Default)]
enum ProgramState {
    #[default]
    NotCompiled,
    Ready(ShaderSet),
    /// Compile/link failed; never retried until the context is rebuilt.
    Failed,
}

/// Lazily compiled pre-5.3 programs (one per renderer / GL context — no global singleton).
#[derive(Debug, Default)]
pub struct ShaderLibrary {
    programs: [ProgramState; 4],
}

impl ShaderLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compiles `which` if it has not been attempted yet; returns whether it is usable. A failure
    /// caused by a lost context is not remembered (it is retried after the restore).
    pub fn ensure(&mut self, gl: &Gl, which: BaseProgram) -> bool {
        let slot = &mut self.programs[which.index()];
        if matches!(slot, ProgramState::NotCompiled) {
            let (vs, fs) = base_program_sources(which);
            let fs = upgrade_fragment_precision(fs);
            *slot = match compile_program(gl, vs, &fs) {
                Ok(program) => ProgramState::Ready(ShaderSet::query(gl, program, which)),
                Err(_) if gl.is_context_lost() => ProgramState::NotCompiled,
                Err(e) => {
                    log::error!("Cubism program {which:?} is unusable ({e}); drawables that need it fall back.");
                    ProgramState::Failed
                }
            };
        }
        matches!(slot, ProgramState::Ready(_))
    }

    /// The program if it is compiled and usable (never compiles).
    pub fn get(&self, which: BaseProgram) -> Option<&ShaderSet> {
        match &self.programs[which.index()] {
            ProgramState::Ready(set) => Some(set),
            _ => None,
        }
    }

    /// `true` once `which` was attempted and failed.
    pub fn has_failed(&self, which: BaseProgram) -> bool {
        matches!(self.programs[which.index()], ProgramState::Failed)
    }

    /// Deletes every program (`releaseShaderProgram`) and returns to the "not compiled" state.
    pub fn release(&mut self, gl: &Gl) {
        for slot in &mut self.programs {
            if let ProgramState::Ready(set) = std::mem::take(slot) {
                gl.delete_program(Some(&set.program));
            }
        }
    }

    /// Forgets every program without GL calls (context lost).
    pub fn forget(&mut self) {
        self.programs = Default::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrades_base_fragment_shaders_to_conditional_highp() {
        for which in [
            BaseProgram::SetupMask,
            BaseProgram::Normal,
            BaseProgram::Masked,
            BaseProgram::MaskedInverted,
        ] {
            let (_, fs) = base_program_sources(which);
            let upgraded = upgrade_fragment_precision(fs);
            assert!(
                matches!(upgraded, Cow::Owned(_)),
                "{which:?} should be upgraded"
            );
            assert!(upgraded.contains("#ifdef GL_FRAGMENT_PRECISION_HIGH\nprecision highp float;"));
            assert!(upgraded.contains("#else\nprecision mediump float;\n#endif"));
        }
        let no_mediump = "void main() { gl_FragColor = vec4(1.0); }";
        assert!(matches!(
            upgrade_fragment_precision(no_mediump),
            Cow::Borrowed(s) if s == no_mediump
        ));
    }
}
