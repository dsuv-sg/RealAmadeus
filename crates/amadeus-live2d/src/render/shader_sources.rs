//! The 13 GLSL ES 1.00 sources of the Cubism Web Framework (`Framework/Shaders/WebGL/`), embedded
//! byte-identically (DECISIONS D4: no runtime fetch). V1 fetched them from
//! `/resources/live2d/shaders/`; the md5 table of render spec §0.2 is verified in the tests.
//!
//! Copyright(c) Live2D Inc. All rights reserved. Live2D Open Software license.

use super::types::{AlphaBlend, ColorBlend, ShaderType};

/// `vertshadersrc.vert` — unmasked drawables.
pub const VERT_SHADER_SRC: &str = include_str!("../../shaders/vertshadersrc.vert");
/// `vertshadersrcmasked.vert` — masked drawables.
pub const VERT_SHADER_SRC_MASKED: &str = include_str!("../../shaders/vertshadersrcmasked.vert");
/// `vertshadersrcsetupmask.vert` — mask generation.
pub const VERT_SHADER_SRC_SETUP_MASK: &str =
    include_str!("../../shaders/vertshadersrcsetupmask.vert");
/// `fragshadersrcsetupmask.frag` — mask generation.
pub const FRAG_SHADER_SRC_SETUP_MASK: &str =
    include_str!("../../shaders/fragshadersrcsetupmask.frag");
/// `fragshadersrcpremultipliedalpha.frag` — unmasked, premultiplied alpha.
pub const FRAG_SHADER_SRC_PREMULTIPLIED_ALPHA: &str =
    include_str!("../../shaders/fragshadersrcpremultipliedalpha.frag");
/// `fragshadersrcmaskpremultipliedalpha.frag` — masked, premultiplied alpha.
pub const FRAG_SHADER_SRC_MASK_PREMULTIPLIED_ALPHA: &str =
    include_str!("../../shaders/fragshadersrcmaskpremultipliedalpha.frag");
/// `fragshadersrcmaskinvertedpremultipliedalpha.frag` — inverted mask, premultiplied alpha.
pub const FRAG_SHADER_SRC_MASK_INVERTED_PREMULTIPLIED_ALPHA: &str =
    include_str!("../../shaders/fragshadersrcmaskinvertedpremultipliedalpha.frag");
/// `vertshadersrccopy.vert` — Cubism 5.3 copy program.
pub const VERT_SHADER_SRC_COPY: &str = include_str!("../../shaders/vertshadersrccopy.vert");
/// `fragshadersrccopy.frag` — Cubism 5.3 copy program.
pub const FRAG_SHADER_SRC_COPY: &str = include_str!("../../shaders/fragshadersrccopy.frag");
/// `fragshadersrccolorblend.frag` — Cubism 5.3 `ColorBlend()` implementations.
pub const FRAG_SHADER_SRC_COLOR_BLEND: &str =
    include_str!("../../shaders/fragshadersrccolorblend.frag");
/// `fragshadersrcalphablend.frag` — Cubism 5.3 `AlphaBlend()` implementations.
pub const FRAG_SHADER_SRC_ALPHA_BLEND: &str =
    include_str!("../../shaders/fragshadersrcalphablend.frag");
/// `vertshadersrcblend.vert` — Cubism 5.3 blend vertex shader.
pub const VERT_SHADER_SRC_BLEND: &str = include_str!("../../shaders/vertshadersrcblend.vert");
/// `fragshadersrcpremultipliedalphablend.frag` — Cubism 5.3 blend fragment shader body.
pub const FRAG_SHADER_SRC_BLEND: &str =
    include_str!("../../shaders/fragshadersrcpremultipliedalphablend.frag");

/// `(file name, source)` in the order of the SDK's constant list / fetch list (render spec §7.1).
pub const ALL_SOURCES: [(&str, &str); 13] = [
    ("vertshadersrc.vert", VERT_SHADER_SRC),
    ("vertshadersrcmasked.vert", VERT_SHADER_SRC_MASKED),
    ("vertshadersrcsetupmask.vert", VERT_SHADER_SRC_SETUP_MASK),
    ("fragshadersrcsetupmask.frag", FRAG_SHADER_SRC_SETUP_MASK),
    (
        "fragshadersrcpremultipliedalpha.frag",
        FRAG_SHADER_SRC_PREMULTIPLIED_ALPHA,
    ),
    (
        "fragshadersrcmaskpremultipliedalpha.frag",
        FRAG_SHADER_SRC_MASK_PREMULTIPLIED_ALPHA,
    ),
    (
        "fragshadersrcmaskinvertedpremultipliedalpha.frag",
        FRAG_SHADER_SRC_MASK_INVERTED_PREMULTIPLIED_ALPHA,
    ),
    ("vertshadersrccopy.vert", VERT_SHADER_SRC_COPY),
    ("fragshadersrccopy.frag", FRAG_SHADER_SRC_COPY),
    ("fragshadersrccolorblend.frag", FRAG_SHADER_SRC_COLOR_BLEND),
    ("fragshadersrcalphablend.frag", FRAG_SHADER_SRC_ALPHA_BLEND),
    ("vertshadersrcblend.vert", VERT_SHADER_SRC_BLEND),
    (
        "fragshadersrcpremultipliedalphablend.frag",
        FRAG_SHADER_SRC_BLEND,
    ),
];

/// `generateBlendShader` source assembly for one Cubism 5.3 combination (render spec §7.3):
/// `fs = "precision mediump float;\n" + colorMacro + alphaMacro + colorblend + alphablend
/// [+ "#define CLIPPING_MASK\n"] + premultipliedalphablend`, `vs = ["#define CLIPPING_MASK\n"] +
/// blend.vert`. Masked and MaskedInverted produce identical sources (inversion is the
/// `u_invertClippingMask` uniform). Returns `(vertex, fragment)`.
pub fn blend_shader_sources(
    color: ColorBlend,
    alpha: AlphaBlend,
    shader_type: ShaderType,
) -> (String, String) {
    let color_macro = format!("#define COLOR_BLEND_{}\n", color.name().to_uppercase());
    let alpha_macro = format!("#define ALPHA_BLEND_{}\n", alpha.name().to_uppercase());
    let mut vertex = String::new();
    let mut fragment = String::from("precision mediump float;\n");
    fragment.push_str(&color_macro);
    fragment.push_str(&alpha_macro);
    fragment.push_str(FRAG_SHADER_SRC_COLOR_BLEND);
    fragment.push_str(FRAG_SHADER_SRC_ALPHA_BLEND);
    if shader_type != ShaderType::Normal {
        vertex.push_str("#define CLIPPING_MASK\n");
        fragment.push_str("#define CLIPPING_MASK\n");
    }
    vertex.push_str(VERT_SHADER_SRC_BLEND);
    fragment.push_str(FRAG_SHADER_SRC_BLEND);
    (vertex, fragment)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal MD5 (RFC 1321) for the provenance check of the embedded files.
    fn md5_hex(data: &[u8]) -> String {
        const S: [u32; 64] = [
            7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20,
            5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
            6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
        ];
        let k: Vec<u32> = (0..64)
            .map(|i| ((i as f64 + 1.0).sin().abs() * 4294967296.0) as u32)
            .collect();
        let (mut a0, mut b0, mut c0, mut d0) = (
            0x67452301_u32,
            0xefcdab89_u32,
            0x98badcfe_u32,
            0x10325476_u32,
        );
        let mut msg = data.to_vec();
        let bit_len = (data.len() as u64).wrapping_mul(8);
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&bit_len.to_le_bytes());
        for chunk in msg.chunks(64) {
            let m: Vec<u32> = chunk
                .chunks(4)
                .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
                .collect();
            let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
            for i in 0..64 {
                let (f, g) = match i / 16 {
                    0 => ((b & c) | (!b & d), i),
                    1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                    2 => (b ^ c ^ d, (3 * i + 5) % 16),
                    _ => (c ^ (b | !d), (7 * i) % 16),
                };
                let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
                a = d;
                d = c;
                c = b;
                b = b.wrapping_add(f.rotate_left(S[i]));
            }
            a0 = a0.wrapping_add(a);
            b0 = b0.wrapping_add(b);
            c0 = c0.wrapping_add(c);
            d0 = d0.wrapping_add(d);
        }
        [a0, b0, c0, d0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    #[test]
    fn md5_implementation_sanity() {
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(
            md5_hex(b"The quick brown fox jumps over the lazy dog"),
            "9e107d9d372bb6826bd81d3542a419d6"
        );
    }

    /// Render spec §0.2: byte-identical to V1 `public/resources/live2d/shaders/`.
    #[test]
    fn embedded_sources_match_v1_md5_table() {
        let expected: [(&str, usize, &str); 13] = [
            (
                "vertshadersrc.vert",
                465,
                "4e1c5fb89f0524e9987747e4f0fe6e15",
            ),
            (
                "vertshadersrcmasked.vert",
                559,
                "eb7251aed7ae67dfaa4e6bee6362a7c2",
            ),
            (
                "vertshadersrcsetupmask.vert",
                536,
                "b2dcf650cbae2ea8dfbc6bd1ea3a2b43",
            ),
            (
                "fragshadersrcsetupmask.frag",
                736,
                "bec774fe1a2d4df16baf5d340c929452",
            ),
            (
                "fragshadersrcpremultipliedalpha.frag",
                747,
                "9b6c1b6774b7e6a4d207df46f7586c18",
            ),
            (
                "fragshadersrcmaskpremultipliedalpha.frag",
                1043,
                "a39e6abc07d37e1fec6a19f810813237",
            ),
            (
                "fragshadersrcmaskinvertedpremultipliedalpha.frag",
                1051,
                "da498fd5b159b0d1c09ca12cb47079f4",
            ),
            (
                "vertshadersrccopy.vert",
                392,
                "ab779e034dfcbe1a5af2873f5f3c518c",
            ),
            (
                "fragshadersrccopy.frag",
                429,
                "fcdda38373872a30cbff7bc7ee7ba67f",
            ),
            (
                "fragshadersrccolorblend.frag",
                7525,
                "8fe459c2c27e2ad4453da78286f2805a",
            ),
            (
                "fragshadersrcalphablend.frag",
                2189,
                "db4b7df97b150e4116bce74e70f86edc",
            ),
            (
                "vertshadersrcblend.vert",
                738,
                "936c093963d5cdb0c1cee3456d4286ec",
            ),
            (
                "fragshadersrcpremultipliedalphablend.frag",
                2038,
                "7a183c23f482ae31c43589377fa3f834",
            ),
        ];
        for ((name, src), (exp_name, exp_len, exp_md5)) in ALL_SOURCES.iter().zip(expected) {
            assert_eq!(*name, exp_name);
            assert_eq!(src.len(), exp_len, "{name} length");
            assert_eq!(md5_hex(src.as_bytes()), exp_md5, "{name} md5");
        }
    }

    #[test]
    fn blend_source_assembly() {
        let (vs, fs) =
            blend_shader_sources(ColorBlend::Multiply, AlphaBlend::Atop, ShaderType::Normal);
        assert_eq!(vs, VERT_SHADER_SRC_BLEND);
        assert!(fs.starts_with(
            "precision mediump float;\n#define COLOR_BLEND_MULTIPLY\n#define ALPHA_BLEND_ATOP\n"
        ));
        assert!(fs.ends_with(FRAG_SHADER_SRC_BLEND));
        assert!(!fs.contains("#define CLIPPING_MASK"));
        let (vm, fm) = blend_shader_sources(
            ColorBlend::LinearLight,
            AlphaBlend::DisjointOver,
            ShaderType::Masked,
        );
        assert!(vm.starts_with("#define CLIPPING_MASK\n"));
        assert!(fm.contains("#define COLOR_BLEND_LINEARLIGHT\n#define ALPHA_BLEND_DISJOINTOVER\n"));
        let expected_tail =
            format!("{FRAG_SHADER_SRC_ALPHA_BLEND}#define CLIPPING_MASK\n{FRAG_SHADER_SRC_BLEND}");
        assert!(fm.ends_with(&expected_tail));
        let inverted = blend_shader_sources(
            ColorBlend::LinearLight,
            AlphaBlend::DisjointOver,
            ShaderType::MaskedInverted,
        );
        assert_eq!(inverted, (vm, fm));
    }
}
