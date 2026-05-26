use crate::script::op_codes::*;
use crate::script::stack::Stack;
use crate::util::ChainGangError;

/// True when the script contains only push operations.
pub fn is_push_only(script: &[u8]) -> bool {
    let mut i = 0;
    while i < script.len() {
        match script[i] {
            OP_0 | OP_1NEGATE | OP_1..=OP_16 | 1..=75 | OP_PUSHDATA1 | OP_PUSHDATA2
            | OP_PUSHDATA4 => {}
            _ => return false,
        }
        i = next_op(i, script);
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
                return Err(ChainGangError::ScriptError(
                    "Non-minimal push".to_string(),
                ));
            }
            if len == 1 {
                match script[i + 1] {
                    0 | 1..=16 | OP_1NEGATE => {
                        return Err(ChainGangError::ScriptError(
                            "Non-minimal push".to_string(),
                        ));
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        OP_PUSHDATA1 => {
            if i + 1 >= script.len() {
                return Ok(());
            }
            if (script[i + 1] as usize) < 76 {
                Err(ChainGangError::ScriptError(
                    "Non-minimal push".to_string(),
                ))
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
                Err(ChainGangError::ScriptError(
                    "Non-minimal push".to_string(),
                ))
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
                Err(ChainGangError::ScriptError(
                    "Non-minimal push".to_string(),
                ))
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
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
