//! ASTからコード生成を行う
use super::{parser::AST, Instruction};
use crate::helper::safe_add;
use std::{
    error::Error,
    fmt::{self, Display},
};

/// コード生成エラーを表す型
#[derive(Debug)]
pub enum CodeGenError {
    PCOverFlow,
    FailStar,
    FailOr,
    FailQuestion,
}

impl Display for CodeGenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CodeGenError: {:?}", self)
    }
}

impl Error for CodeGenError {}

/// コード生成器
#[derive(Default, Debug)]
struct Generator {
    pc: usize,
    insts: Vec<Instruction>,
}

/// コード生成を行う関数
pub fn gen_code(ast: &AST) -> Result<Vec<Instruction>, CodeGenError> {
    let mut generator = Generator::default();
    generator.gen_code(ast)?;
    Ok(generator.insts)
}

/// コード生成器のメソッド定義
impl Generator {
    /// コード生成を行う関数の入り口
    fn gen_code(&mut self, ast: &AST) -> Result<(), CodeGenError> {
        self.gen_expr(ast)?;
        self.inc_pc()?;
        self.insts.push(Instruction::Match);
        Ok(())
    }

    /// ASTをパターン分けし、コード生成を行う関数
    fn gen_expr(&mut self, ast: &AST) -> Result<(), CodeGenError> {
        match ast {
            AST::Char(c) => self.gen_char(*c)?,
            AST::Or(e1, e2) => self.gen_or(e1, e2)?,
            AST::Plus(e) => self.gen_plus(e)?,
            AST::Star(e1) => {
                match &**e1 {
                    // `(a*)*`のように`Star`が二重になっている場合にスタックオーバーフローする問題を回避するため、
                    // このような`(((r*)*)*...*)*`を再帰的に処理して1つの`r*`へと変換する。
                    AST::Star(_) => self.gen_expr(&e1)?,
                    AST::Seq(e2) if e2.len() == 1 => {
                        if let Some(e3 @ AST::Star(_)) = e2.get(0) {
                            self.gen_expr(e3)?
                        } else {
                            self.gen_star(e1)?
                        }
                    }
                    e => self.gen_star(&e)?,
                }
            }
            AST::Question(e) => self.gen_question(e)?,
            AST::Seq(v) => self.gen_seq(v)?,
            AST::Dot => {
                self.insts.push(Instruction::AnyChar);
                self.inc_pc()?;
            }
        }

        Ok(())
    }

    /// char命令生成関数
    fn gen_char(&mut self, c: char) -> Result<(), CodeGenError> {
        let inst = Instruction::Char(c);
        self.insts.push(inst);
        self.inc_pc()?;
        Ok(())
    }

    /// OR演算子のコード生成を行う関数
    ///
    /// 以下のようなコードを生成
    ///
    /// ```text
    ///    split L1, L2
    /// L1: e1のコード
    ///     jump L3
    /// L2: e2のコード
    /// L3:
    /// ```
    fn gen_or(&mut self, e1: &AST, e2: &AST) -> Result<(), CodeGenError> {
        // split L1, L2
        let split_addr = self.pc;
        self.inc_pc()?;
        let split = Instruction::Split(self.pc, 0);
        self.insts.push(split);

        // L1: e1のコード
        self.gen_expr(e1)?;

        // jump L3
        let jump_addr = self.pc;
        self.insts.push(Instruction::Jump(0));

        // L2の値を設定
        self.inc_pc()?;
        if let Some(Instruction::Split(_, l2)) = self.insts.get_mut(split_addr) {
            *l2 = self.pc;
        } else {
            return Err(CodeGenError::FailOr);
        }

        // L2: e2のコード
        self.gen_expr(e2)?;

        // L3の値を設定
        if let Some(Instruction::Jump(l3)) = self.insts.get_mut(jump_addr) {
            *l3 = self.pc;
        } else {
            return Err(CodeGenError::FailOr);
        }

        Ok(())
    }

    /// ?限量子のコード生成を行う関数
    ///
    /// 以下のようなコードを生成
    ///
    /// ```text
    ///     split L1, L2
    /// L1: eのコード
    /// L2:
    /// ```
    fn gen_question(&mut self, e: &AST) -> Result<(), CodeGenError> {
        // split L1, L2
        let split_addr = self.pc;
        self.inc_pc()?;
        let split = Instruction::Split(self.pc, 0);
        self.insts.push(split);

        // L1: eのコード
        self.gen_expr(e)?;

        // L2の値を設定
        if let Some(Instruction::Split(_, l2)) = self.insts.get_mut(split_addr) {
            *l2 = self.pc;
            Ok(())
        } else {
            Err(CodeGenError::FailQuestion)
        }
    }

    /// +限量子のコード生成を行う関数
    ///
    /// 以下のようなコードを生成
    ///
    /// ```text
    /// L1: eのコード
    ///     split L1, L2
    /// L2:
    /// ```
    fn gen_plus(&mut self, e: &AST) -> Result<(), CodeGenError> {
        // L1: eのコード
        let l1 = self.pc;
        self.gen_expr(e)?;

        // split L1, L2
        self.inc_pc()?;
        let split = Instruction::Split(l1, self.pc);
        self.insts.push(split);

        Ok(())
    }

    /// *限量子のコード生成を行う関数
    ///
    /// 以下のようなコードを生成
    ///
    /// ```text
    /// L1: split L2, L3
    /// L2: eのコード
    ///     jump L1
    /// L3:
    /// ```
    fn gen_star(&mut self, e: &AST) -> Result<(), CodeGenError> {
        // L1: split L2, L3
        let l1 = self.pc;
        self.inc_pc()?;
        let split = Instruction::Split(self.pc, 0);
        self.insts.push(split);

        // L2: eのコード
        self.gen_expr(e)?;

        // jump L1
        self.inc_pc()?;
        self.insts.push(Instruction::Jump(l1));

        // L3の値を設定
        if let Some(Instruction::Split(_, l3)) = self.insts.get_mut(l1) {
            *l3 = self.pc;
            Ok(())
        } else {
            Err(CodeGenError::FailStar)
        }
    }

    /// シーケンスのコード生成を行う関数
    fn gen_seq(&mut self, exprs: &[AST]) -> Result<(), CodeGenError> {
        for e in exprs {
            self.gen_expr(e)?;
        }

        Ok(())
    }

    /// プログラムカウンタをインクリメントする関数
    fn inc_pc(&mut self) -> Result<(), CodeGenError> {
        safe_add(&mut self.pc, &1, || CodeGenError::PCOverFlow)
    }
}

/// コード生成のテスト
#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::parser::parse;

    #[test]
    fn test_code_gen() {
        // 正常系
        assert_eq!(
            gen_code(&parse("abc").unwrap()).unwrap(),
            vec![
                Instruction::Char('a'),
                Instruction::Char('b'),
                Instruction::Char('c'),
                Instruction::Match,
            ]
        );
        assert_eq!(
            gen_code(&parse("ab|c").unwrap()).unwrap(),
            vec![
                Instruction::Split(1, 4),
                Instruction::Char('a'),
                Instruction::Char('b'),
                Instruction::Jump(5),
                Instruction::Char('c'),
                Instruction::Match,
            ]
        );
        assert_eq!(
            gen_code(&parse("(ab)+c").unwrap()).unwrap(),
            vec![
                Instruction::Char('a'),
                Instruction::Char('b'),
                Instruction::Split(0, 3),
                Instruction::Char('c'),
                Instruction::Match,
            ]
        );
        assert_eq!(
            gen_code(&parse("(ab)?c").unwrap()).unwrap(),
            vec![
                Instruction::Split(1, 3),
                Instruction::Char('a'),
                Instruction::Char('b'),
                Instruction::Char('c'),
                Instruction::Match,
            ]
        );
        assert_eq!(
            gen_code(&parse("(ab)*c").unwrap()).unwrap(),
            vec![
                Instruction::Split(1, 4),
                Instruction::Char('a'),
                Instruction::Char('b'),
                Instruction::Jump(0),
                Instruction::Char('c'),
                Instruction::Match,
            ]
        );
    }
}
