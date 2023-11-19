use nom::{
    branch::alt,
    character::complete::{char, one_of},
    error::ErrorKind,
    multi::{many0, many1},
    IResult,
};
use rustyline::Editor;

#[derive(Debug)]
enum Expr {
    Num(i64),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
}

fn main() {
    let mut rl = Editor::<()>::new().unwrap();

    loop {
        if let Ok(readline) = rl.readline(">> ") {
            if let Some(e) = parse(&readline) {
                println!("result: {}", eval(&e));
            }
        } else {
            break;
        }
    }
}

fn parse(c: &str) -> Option<Expr> {
    match parse_expr(c) {
        Ok((_, e)) => {
            println!("AST: {:?}", e);
            Some(e)
        }
        Err(e) => {
            println!("Error: {:?}", e);
            None
        }
    }
}

fn parse_expr(c: &str) -> IResult<&str, Expr> {
    let (c, _) = many0(char(' '))(c)?;

    let result = alt((parse_num, parse_op))(c)?;
    Ok(result)
}

fn parse_num(c: &str) -> IResult<&str, Expr> {
    let (c1, v) = many1(one_of("0123456789"))(c)?;
    let var: String = v.into_iter().collect();

    if let Ok(n) = var.parse::<i64>() {
        Ok((c1, Expr::Num(n)))
    } else {
        let err = nom::error::Error::new(c, ErrorKind::Fail);
        Err(nom::Err::Failure(err))
    }
}

fn parse_op(c: &str) -> IResult<&str, Expr> {
    let (c, op) = one_of("+-*")(c)?;
    let (c, e1) = parse_expr(c)?;
    let (c, e2) = parse_expr(c)?;

    match op {
        '+' => Ok((c, Expr::Add(Box::new(e1), Box::new(e2)))),
        '-' => Ok((c, Expr::Sub(Box::new(e1), Box::new(e2)))),
        '*' => Ok((c, Expr::Mul(Box::new(e1), Box::new(e2)))),
        _ => {
            let err = nom::error::Error::new(c, ErrorKind::Fail);
            Err(nom::Err::Failure(err))
        }
    }
}

fn eval(e: &Expr) -> i64 {
    match e {
        Expr::Num(n) => *n,
        Expr::Add(a, b) => eval(a) + eval(b),
        Expr::Sub(a, b) => eval(a) - eval(b),
        Expr::Mul(a, b) => eval(a) * eval(b),
    }
}
