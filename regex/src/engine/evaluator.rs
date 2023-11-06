//! 命令列と入力文字列を受け取り、マッチングを行う

use super::Instruction;
use crate::helper::safe_add;
use std::{
    error::Error,
    fmt::{self, Display},
};

#[derive(Debug)]
pub enum EvalError {
    PCOverFlow,
    SPOverFlow,
    InvalidPC,
}

impl Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EvalError: {:?}", self)
    }
}

impl Error for EvalError {}

/// 命令列の評価を行う関数
///
/// instが命令列となり、その命令列を用いて入力文字列lineにマッチさせる
pub fn eval(
    inst: &[Instruction],
    line: &[char],
    include_head_of_line: bool,
) -> Result<bool, EvalError> {
    eval_depth(inst, line, 0, 0, include_head_of_line)
}

/// 深さ優先探索で再起的にマッチングを行う評価関数
fn eval_depth(
    inst: &[Instruction],
    line: &[char],
    mut pc: usize,
    mut sp: usize,
    include_head_of_line: bool,
) -> Result<bool, EvalError> {
    loop {
        let next = if let Some(i) = inst.get(pc) {
            i
        } else {
            return Err(EvalError::InvalidPC);
        };

        match next {
            Instruction::Char(c) => {
                if let Some(sp_c) = line.get(sp) {
                    if c == sp_c {
                        safe_add(&mut pc, &1, || EvalError::PCOverFlow)?;
                        safe_add(&mut sp, &1, || EvalError::SPOverFlow)?;
                    } else {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }
            }
            Instruction::AnyChar => {
                safe_add(&mut pc, &1, || EvalError::PCOverFlow)?;
                safe_add(&mut sp, &1, || EvalError::SPOverFlow)?;
            }
            Instruction::HeadOfLine => {
                if !include_head_of_line || sp != 0 {
                    return Ok(false);
                }
                safe_add(&mut pc, &1, || EvalError::PCOverFlow)?;
            }
            Instruction::EndOfLine => {
                if sp != line.len() {
                    return Ok(false);
                }
                safe_add(&mut pc, &1, || EvalError::PCOverFlow)?;
            }
            Instruction::Jump(addr) => {
                pc = *addr;
            }
            Instruction::Split(addr1, addr2) => {
                if eval_depth(inst, line, *addr1, sp, include_head_of_line)?
                    || eval_depth(inst, line, *addr2, sp, include_head_of_line)?
                {
                    return Ok(true);
                } else {
                    return Ok(false);
                }
            }
            Instruction::Match => {
                return Ok(true);
            }
        }
    }
}
