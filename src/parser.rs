use crate::{expression::ExprId, runner::LazyKRunner};
use anyhow::{bail, ensure, Result};

pub struct Parser {}

impl Parser {
    fn parse_jot(source: &mut &[u8], pool: &mut LazyKRunner) -> ExprId {
        let mut e = pool.i;
        let mut i = 0;
        while i != source.len() {
            if source[i] == b'0' {
                let lhs = pool.partial_apply(e, pool.s);
                e = pool.partial_apply(lhs, pool.k);
            } else if source[i] == b'1' {
                let rhs = pool.partial_apply(pool.k, e);
                e = pool.partial_apply(pool.s, rhs);
            }
            i += 1;
        }
        *source = &source[i..];
        e
    }

    fn skip_whitespace_and_comments(source: &mut &[u8]) {
        let mut is_comment = false;
        for i in 0..source.len() {
            if source[i] >= 128 {
                continue;
            }
            let ch = source[i] as char;
            if ch == '#' {
                is_comment = true;
            }
            if ch == '\n' {
                is_comment = false;
            }
            if ch <= ' ' || is_comment {
                continue;
            }
            *source = &source[i..];
            return;
        }
        *source = &source[source.len()..];
    }

    fn parse_expr(source: &mut &[u8], i_is_iota: bool, pool: &mut LazyKRunner) -> Result<ExprId> {
        Self::skip_whitespace_and_comments(source);
        if source.is_empty() {
            bail!("Unexpected end of source.")
        }
        let ch = source[0] as char;
        if ch == '0' || ch == '1' {
            return Ok(Self::parse_jot(source, pool));
        }

        *source = &source[1..];
        match ch {
            '`' | '*' => {
                let p = Self::parse_expr(source, ch == '*', pool)?;
                let q = Self::parse_expr(source, ch == '*', pool)?;
                Ok(pool.partial_apply(p, q))
            }
            '(' => Self::parse_manual_close(source, true, pool),
            ')' => bail!("Mismatched close-parenthesis!"),
            'k' | 'K' => Ok(pool.k),
            's' | 'S' => Ok(pool.s),
            'i' => {
                if i_is_iota {
                    Ok(pool.iota)
                } else {
                    Ok(pool.i)
                }
            }
            'I' => Ok(pool.i),
            _ => bail!("Invalid character: [{}]", ch),
        }
    }

    fn parse_manual_close(
        source: &mut &[u8],
        expected_closing_paren: bool,
        pool: &mut LazyKRunner,
    ) -> Result<ExprId> {
        let mut e: Option<ExprId> = None;
        loop {
            Self::skip_whitespace_and_comments(source);
            if source.is_empty() || source[0] == b')' {
                break;
            }
            let e2 = Self::parse_expr(source, false, pool)?;
            e = match e {
                Some(e) => Some(pool.partial_apply(e, e2)),
                None => Some(e2),
            }
        }
        if expected_closing_paren {
            ensure!(!source.is_empty(), "Premature end of program.");
            *source = &source[1..];
        } else {
            ensure!(source.is_empty(), "Unmatched trailing close-parenthesis.");
        }
        match e {
            Some(e) => Ok(e),
            None => Ok(pool.i),
        }
    }

    pub fn parse(source: &str, pool: &mut LazyKRunner) -> Result<ExprId> {
        let mut b = source.as_bytes();
        Self::parse_manual_close(&mut b, false, pool)
    }
}
