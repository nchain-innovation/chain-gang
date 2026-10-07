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
        let end = match script[i] {
            OP_0 | OP_1NEGATE | OP_1..=OP_16 => i + 1,
            len @ 1..=75 => i + 1 + len as usize,
            OP_PUSHDATA1 => match script.get(i + 1) {
                Some(&len) => i + 2 + len as usize,
                None => return false,
            },
            OP_PUSHDATA2 => match script.get(i + 1..i + 3) {
                Some(len) => i + 3 + u16::from_le_bytes([len[0], len[1]]) as usize,
                None => return false,
            },
            OP_PUSHDATA4 => match script.get(i + 1..i + 5) {
                Some(len) => i + 5 + u32::from_le_bytes([len[0], len[1], len[2], len[3]]) as usize,
                None => return false,
            },
            _ => return false,
        };
        if end > script.len() {
            return false;
        }
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
    if i >= script.len() {
        return script.len();
    }
    let next = match script[i] {
        len @ 1..=75 => i + 1 + len as usize,
        OP_PUSHDATA1 => {
            if i + 2 > script.len() {
                return script.len();
            }
            i + 2 + script[i + 1] as usize
        }
        OP_PUSHDATA2 => {
            if i + 3 > script.len() {
                return script.len();
            }
            i + 3 + (script[i + 1] as usize) + ((script[i + 2] as usize) << 8)
        }
        OP_PUSHDATA4 => {
            if i + 5 > script.len() {
                return script.len();
            }
            let len = (script[i + 1] as usize)
                + ((script[i + 2] as usize) << 8)
                + ((script[i + 3] as usize) << 16)
                + ((script[i + 4] as usize) << 24);
            i + 5 + len
        }
        _ => i + 1,
    };
    let overflow = next > script.len();
    if overflow {
        script.len()
    } else {
        next
    }
}

/// Skips over a branch of if/else and return the index of the next else or endif opcode
pub(crate) fn skip_branch(script: &[u8], mut i: usize) -> usize {
    let mut sub = 0;
    while i < script.len() {
        match script[i] {
            OP_IF => sub += 1,
            OP_NOTIF => sub += 1,
            OP_VERIF => sub += 1,
            OP_VERNOTIF => sub += 1,
            OP_ELSE => {
                if sub == 0 {
                    return i;
                }
            }
            OP_ENDIF => {
                if sub == 0 {
                    return i;
                }
                sub -= 1;
            }
            _ => {}
        }
        i = next_op(i, script);
    }
    script.len()
}
