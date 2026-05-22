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

/// Returns the canonical opcode name, if `byte` is a known opcode (not a direct push).
pub fn opcode_name(byte: u8) -> Option<&'static str> {
    match byte {
        OP_0 => Some("OP_0"),
        OP_1NEGATE => Some("OP_1NEGATE"),
        OP_1 => Some("OP_1"),
        OP_2 => Some("OP_2"),
        OP_3 => Some("OP_3"),
        OP_4 => Some("OP_4"),
        OP_5 => Some("OP_5"),
        OP_6 => Some("OP_6"),
        OP_7 => Some("OP_7"),
        OP_8 => Some("OP_8"),
        OP_9 => Some("OP_9"),
        OP_10 => Some("OP_10"),
        OP_11 => Some("OP_11"),
        OP_12 => Some("OP_12"),
        OP_13 => Some("OP_13"),
        OP_14 => Some("OP_14"),
        OP_15 => Some("OP_15"),
        OP_16 => Some("OP_16"),
        OP_NOP => Some("OP_NOP"),
        OP_VER => Some("OP_VER"),
        OP_IF => Some("OP_IF"),
        OP_NOTIF => Some("OP_NOTIF"),
        OP_VERIF => Some("OP_VERIF"),
        OP_VERNOTIF => Some("OP_VERNOTIF"),
        OP_ELSE => Some("OP_ELSE"),
        OP_ENDIF => Some("OP_ENDIF"),
        OP_VERIFY => Some("OP_VERIFY"),
        OP_RETURN => Some("OP_RETURN"),
        OP_TOALTSTACK => Some("OP_TOALTSTACK"),
        OP_FROMALTSTACK => Some("OP_FROMALTSTACK"),
        OP_IFDUP => Some("OP_IFDUP"),
        OP_DEPTH => Some("OP_DEPTH"),
        OP_DROP => Some("OP_DROP"),
        OP_DUP => Some("OP_DUP"),
        OP_NIP => Some("OP_NIP"),
        OP_OVER => Some("OP_OVER"),
        OP_PICK => Some("OP_PICK"),
        OP_ROLL => Some("OP_ROLL"),
        OP_ROT => Some("OP_ROT"),
        OP_SWAP => Some("OP_SWAP"),
        OP_TUCK => Some("OP_TUCK"),
        OP_2DROP => Some("OP_2DROP"),
        OP_2DUP => Some("OP_2DUP"),
        OP_3DUP => Some("OP_3DUP"),
        OP_2OVER => Some("OP_2OVER"),
        OP_2ROT => Some("OP_2ROT"),
        OP_2SWAP => Some("OP_2SWAP"),
        OP_CAT => Some("OP_CAT"),
        OP_SPLIT => Some("OP_SPLIT"),
        OP_SUBSTR => Some("OP_SUBSTR"),
        OP_LEFT => Some("OP_LEFT"),
        OP_RIGHT => Some("OP_RIGHT"),
        OP_SIZE => Some("OP_SIZE"),
        OP_AND => Some("OP_AND"),
        OP_OR => Some("OP_OR"),
        OP_XOR => Some("OP_XOR"),
        OP_EQUAL => Some("OP_EQUAL"),
        OP_EQUALVERIFY => Some("OP_EQUALVERIFY"),
        OP_1ADD => Some("OP_1ADD"),
        OP_1SUB => Some("OP_1SUB"),
        OP_NEGATE => Some("OP_NEGATE"),
        OP_ABS => Some("OP_ABS"),
        OP_NOT => Some("OP_NOT"),
        OP_0NOTEQUAL => Some("OP_0NOTEQUAL"),
        OP_ADD => Some("OP_ADD"),
        OP_SUB => Some("OP_SUB"),
        OP_MUL => Some("OP_MUL"),
        OP_2MUL => Some("OP_2MUL"),
        OP_DIV => Some("OP_DIV"),
        OP_2DIV => Some("OP_2DIV"),
        OP_MOD => Some("OP_MOD"),
        OP_LSHIFT => Some("OP_LSHIFT"),
        OP_RSHIFT => Some("OP_RSHIFT"),
        OP_LSHIFTNUM => Some("OP_LSHIFTNUM"),
        OP_RSHIFTNUM => Some("OP_RSHIFTNUM"),
        OP_BOOLAND => Some("OP_BOOLAND"),
        OP_BOOLOR => Some("OP_BOOLOR"),
        OP_NUMEQUAL => Some("OP_NUMEQUAL"),
        OP_NUMEQUALVERIFY => Some("OP_NUMEQUALVERIFY"),
        OP_NUMNOTEQUAL => Some("OP_NUMNOTEQUAL"),
        OP_LESSTHAN => Some("OP_LESSTHAN"),
        OP_GREATERTHAN => Some("OP_GREATERTHAN"),
        OP_LESSTHANOREQUAL => Some("OP_LESSTHANOREQUAL"),
        OP_GREATERTHANOREQUAL => Some("OP_GREATERTHANOREQUAL"),
        OP_MIN => Some("OP_MIN"),
        OP_MAX => Some("OP_MAX"),
        OP_WITHIN => Some("OP_WITHIN"),
        OP_NUM2BIN => Some("OP_NUM2BIN"),
        OP_BIN2NUM => Some("OP_BIN2NUM"),
        OP_RIPEMD160 => Some("OP_RIPEMD160"),
        OP_SHA1 => Some("OP_SHA1"),
        OP_SHA256 => Some("OP_SHA256"),
        OP_HASH160 => Some("OP_HASH160"),
        OP_HASH256 => Some("OP_HASH256"),
        OP_CODESEPARATOR => Some("OP_CODESEPARATOR"),
        OP_CHECKSIG => Some("OP_CHECKSIG"),
        OP_CHECKSIGVERIFY => Some("OP_CHECKSIGVERIFY"),
        OP_CHECKMULTISIG => Some("OP_CHECKMULTISIG"),
        OP_CHECKMULTISIGVERIFY => Some("OP_CHECKMULTISIGVERIFY"),
        OP_CHECKLOCKTIMEVERIFY => Some("OP_CHECKLOCKTIMEVERIFY"),
        OP_CHECKSEQUENCEVERIFY => Some("OP_CHECKSEQUENCEVERIFY"),
        _ => None,
    }
}

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
