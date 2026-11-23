use crate::script::op_codes::*;
use crate::script::stack::Stack;
use crate::util::ChainGangError;

use super::MAX_SCRIPT_ELEMENT_SIZE_PREGENESIS;

/// True when the script contains only push operations.
///
/// A push whose data runs past the end of the script is not a push: the node's
/// `GetOp` fails on it, so its `IsPushOnly` is false and evaluating it is
/// `BAD_OPCODE`. This used to step over the missing bytes and return true, so an
/// unlocking script such as `0x1a`, a push of 26 bytes with none present,
/// counted as push-only.
pub fn is_push_only(script: &[u8]) -> bool {
    let mut i = 0;
    while i < script.len() {
        // IsPushOnly includes OP_RESERVED, although executing it fails.
        if script[i] > OP_16 {
            return false;
        }
        let Ok(end) = next_op_cursor(i, script) else {
            return false;
        };
        i = end;
    }
    true
}

pub(crate) fn check_canonical_push(i: usize, script: &[u8]) -> Result<(), ChainGangError> {
    let op = script[i];
    match op {
        OP_0 => Ok(()),
        1..=75 => {
            let len = op as usize;
            if len == 0 {
                return Err(ChainGangError::ScriptError("Non-minimal push".to_string()));
            }
            // The node's CheckMinimalPush: a single byte that an opcode can
            // push on its own must use it. That is 1..=16 (OP_1..OP_16) and
            // 0x81, the value OP_1NEGATE pushes. 0x00 has no such opcode, since
            // OP_0 pushes empty data, not a zero byte. This used to list 0 and
            // compare against OP_1NEGATE (0x4f) itself, so it rejected 0x00
            // and 0x4f and let 0x81 through (#203).
            if len == 1 {
                if let 1..=16 | 0x81 = script[i + 1] {
                    return Err(ChainGangError::ScriptError("Non-minimal push".to_string()));
                }
            }
            Ok(())
        }
        OP_PUSHDATA1 => {
            if i + 1 >= script.len() {
                return Ok(());
            }
            if (script[i + 1] as usize) < 76 {
                Err(ChainGangError::ScriptError("Non-minimal push".to_string()))
            } else {
                Ok(())
            }
        }
        OP_PUSHDATA2 => {
            if i + 2 >= script.len() {
                return Ok(());
            }
            let len = (script[i + 1] as usize) + ((script[i + 2] as usize) << 8);
            if len <= 255 {
                Err(ChainGangError::ScriptError("Non-minimal push".to_string()))
            } else {
                Ok(())
            }
        }
        OP_PUSHDATA4 => {
            if i + 4 >= script.len() {
                return Ok(());
            }
            let len = (script[i + 1] as usize)
                + ((script[i + 2] as usize) << 8)
                + ((script[i + 3] as usize) << 16)
                + ((script[i + 4] as usize) << 24);
            if len <= 65535 {
                Err(ChainGangError::ScriptError("Non-minimal push".to_string()))
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}

/// Fails a push of more than the node's pre-Genesis element limit, taking the
/// length the push opcode at `i` declares.
///
/// The node checks each push as it reads it, before it decides whether the
/// branch is executing, so this applies to pushes that are skipped too. A
/// push whose data runs past the end of the script fails either way.
pub(crate) fn check_pregenesis_push_size(i: usize, script: &[u8]) -> Result<(), ChainGangError> {
    let len = match script[i] {
        len @ 1..=75 => len as usize,
        OP_PUSHDATA1 => script.get(i + 1).map_or(0, |&len| len as usize),
        OP_PUSHDATA2 => script
            .get(i + 1..i + 3)
            .map_or(0, |len| u16::from_le_bytes([len[0], len[1]]) as usize),
        OP_PUSHDATA4 => script.get(i + 1..i + 5).map_or(0, |len| {
            u32::from_le_bytes([len[0], len[1], len[2], len[3]]) as usize
        }),
        _ => 0,
    };
    if len > MAX_SCRIPT_ELEMENT_SIZE_PREGENESIS {
        return Err(ChainGangError::ScriptError(format!(
            "Push of {len} bytes exceeds the pre-Genesis limit of {MAX_SCRIPT_ELEMENT_SIZE_PREGENESIS}"
        )));
    }
    Ok(())
}

#[inline]
pub(crate) fn check_stack_size(minsize: usize, stack: &Stack) -> Result<(), ChainGangError> {
    if stack.len() < minsize {
        return Err(ChainGangError::ScriptError(format!(
            "Stack too small: {minsize}"
        )));
    }
    Ok(())
}

#[inline]
pub(crate) fn remains(i: usize, len: usize, script: &[u8]) -> Result<(), ChainGangError> {
    if i + len > script.len() {
        Err(ChainGangError::ScriptError(
            "Not enough data remaining".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// Gets the next operation index in the script, or the script length if at the end
pub fn next_op(i: usize, script: &[u8]) -> usize {
    next_op_cursor(i, script).unwrap_or(script.len())
}

/// Read an instruction like BSV GetOp. On a truncated push, return the
/// cursor after the opcode and any complete length field, before its data.
pub(crate) fn next_op_cursor(i: usize, script: &[u8]) -> Result<usize, usize> {
    let Some(&opcode) = script.get(i) else {
        return Err(script.len());
    };
    let header = i + 1;
    let (cursor, len) = match opcode {
        1..=75 => (header, usize::from(opcode)),
        OP_PUSHDATA1 => {
            let len = *script.get(header).ok_or(header)?;
            (header + 1, usize::from(len))
        }
        OP_PUSHDATA2 => {
            let bytes = script.get(header..header + 2).ok_or(header)?;
            (
                header + 2,
                usize::from(u16::from_le_bytes([bytes[0], bytes[1]])),
            )
        }
        OP_PUSHDATA4 => {
            let bytes = script.get(header..header + 4).ok_or(header)?;
            let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            (header + 4, len as usize)
        }
        _ => return Ok(header),
    };
    if len > script.len() - cursor {
        Err(cursor)
    } else {
        Ok(cursor + len)
    }
}
