use crate::script::op_codes::*;
use crate::script::interpreter::next_op;
use hex;

/// How to render push-data and unknown opcodes when formatting a script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScriptFormatStyle {
    /// Human-readable form used by `Script::string_representation`.
    StringRep { include_byte_offsets: bool },
    /// Compact form used by `Debug for Script`.
    Debug,
}

// `opcode_name` is generated from the opcode constants in `op_codes.rs` and imported
// via the glob `use` above, so the byte->name table lives in a single place.

fn append_direct_push(
    out: &mut String,
    script: &[u8],
    i: usize,
    len: u8,
    style: ScriptFormatStyle,
) -> bool {
    let data_end = i + 1 + len as usize;
    if data_end > script.len() {
        return false;
    }

    match style {
        ScriptFormatStyle::StringRep {
            include_byte_offsets,
        } => {
            out.push_str("0x");
            if include_byte_offsets {
                out.push_str(&hex::encode(&script[i..data_end]));
            } else {
                out.push_str(&hex::encode(&script[i + 1..data_end]));
            }
        }
        ScriptFormatStyle::Debug => {
            out.push_str(&format!("OP_PUSH+{len} "));
            out.push_str(&hex::encode(&script[i + 1..data_end]));
        }
    }
    true
}

fn append_pushdata1(out: &mut String, script: &[u8], i: usize, style: ScriptFormatStyle) -> bool {
    out.push_str("OP_PUSHDATA1 ");
    if i + 2 > script.len() {
        return false;
    }
    let len = script[i + 1] as usize;
    match style {
        ScriptFormatStyle::StringRep { .. } => out.push_str(&format!("{len:#04x} ")),
        ScriptFormatStyle::Debug => out.push_str(&format!("{len} ")),
    }
    if i + 2 + len > script.len() {
        return false;
    }
    match style {
        ScriptFormatStyle::StringRep { .. } => out.push_str("0x"),
        ScriptFormatStyle::Debug => {}
    }
    out.push_str(&hex::encode(&script[i + 2..i + 2 + len]));
    true
}

fn append_pushdata2(out: &mut String, script: &[u8], i: usize, style: ScriptFormatStyle) -> bool {
    out.push_str("OP_PUSHDATA2 ");
    if i + 3 > script.len() {
        return false;
    }
    let len = (script[i + 1] as usize) + ((script[i + 2] as usize) << 8);
    match style {
        ScriptFormatStyle::StringRep { .. } => {
            out.push_str(&format!(
                "{:#04x}{:02x} ",
                script[i + 1] as usize,
                script[i + 2] as usize
            ));
        }
        ScriptFormatStyle::Debug => out.push_str(&format!("{len} ")),
    }
    if i + 3 + len > script.len() {
        return false;
    }
    match style {
        ScriptFormatStyle::StringRep { .. } => out.push_str("0x"),
        ScriptFormatStyle::Debug => {}
    }
    out.push_str(&hex::encode(&script[i + 3..i + 3 + len]));
    true
}

fn append_pushdata4(out: &mut String, script: &[u8], i: usize, style: ScriptFormatStyle) -> bool {
    out.push_str("OP_PUSHDATA4 ");
    if i + 5 > script.len() {
        return false;
    }
    let len = (script[i + 1] as usize)
        + ((script[i + 2] as usize) << 8)
        + ((script[i + 3] as usize) << 16)
        + ((script[i + 4] as usize) << 24);
    match style {
        ScriptFormatStyle::StringRep { .. } => {
            out.push_str(&format!(
                "{:#04x}{:02x}{:02x}{:02x} ",
                script[i + 1] as usize,
                script[i + 2] as usize,
                script[i + 3] as usize,
                script[i + 4] as usize
            ));
        }
        ScriptFormatStyle::Debug => out.push_str(&format!("{len} ")),
    }
    if i + 5 + len > script.len() {
        return false;
    }
    match style {
        ScriptFormatStyle::StringRep { .. } => out.push_str("0x"),
        ScriptFormatStyle::Debug => {}
    }
    out.push_str(&hex::encode(&script[i + 5..i + 5 + len]));
    true
}

fn append_script_op(
    out: &mut String,
    script: &[u8],
    i: usize,
    style: ScriptFormatStyle,
) -> bool {
    match script[i] {
        len @ 1..=75 => append_direct_push(out, script, i, len, style),
        OP_PUSHDATA1 => append_pushdata1(out, script, i, style),
        OP_PUSHDATA2 => append_pushdata2(out, script, i, style),
        OP_PUSHDATA4 => append_pushdata4(out, script, i, style),
        byte => {
            if let Some(name) = opcode_name(byte) {
                out.push_str(name);
            } else {
                out.push_str(&byte.to_string());
            }
            true
        }
    }
}

pub(crate) fn format_script(script: &[u8], style: ScriptFormatStyle, prefix: &str, suffix: &str) -> String {
    let mut ret = String::new();
    ret.push_str(prefix);
    let mut i = 0;

    while i < script.len() {
        if i != 0 {
            ret.push(' ');
        }
        if !append_script_op(&mut ret, script, i, style) {
            break;
        }
        i = next_op(i, script);
    }

    if i < script.len() {
        for item in script.iter().skip(i) {
            ret.push_str(&format!(" {item}"));
        }
    }

    ret.push_str(suffix);
    ret
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::Script;

    #[test]
    fn opcode_name_covers_chronicle_opcodes() {
        assert_eq!(opcode_name(OP_VER), Some("OP_VER"));
        assert_eq!(opcode_name(OP_SUBSTR), Some("OP_SUBSTR"));
        assert_eq!(opcode_name(OP_LSHIFTNUM), Some("OP_LSHIFTNUM"));
    }

    #[test]
    fn format_script_matches_string_representation() {
        let mut script = Script::new();
        script.append_slice(&[OP_10, OP_5, OP_DIV]);
        assert_eq!(
            script.string_representation(false),
            format_script(&script.0, ScriptFormatStyle::StringRep { include_byte_offsets: false }, "", "")
        );
    }

    #[test]
    fn format_script_matches_debug() {
        let mut script = Script::new();
        script.append_slice(&[OP_10, OP_5, OP_DIV]);
        assert_eq!(
            format!("{script:?}"),
            format_script(&script.0, ScriptFormatStyle::Debug, "[", "]")
        );
    }
}
