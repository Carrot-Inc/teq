//! The positions of ScalametaParser.scala's "position handling" section: a node's tokens from the current token at
//! its production's entry to the last it consumed, trimmed by `atPosWithBody`, which captures its comments too.

use super::{Parser, P};
use crate::fix::scanner::C;

impl<'a> Parser<'a> {
    fn tc(&self, k: i32) -> Option<C> {
        (k >= 0 && (k as usize) < self.s.tokens.len()).then(|| self.s.tokens[k as usize].c)
    }

    /// `tokens.rskipIf(p, beg, end)`: back from `beg` while `p` holds, down to `end`, which it stops at.
    fn rskip_if(&self, p: impl Fn(C) -> bool, beg: i32, end: i32) -> i32 {
        if beg >= self.s.tokens.len() as i32 {
            return beg;
        }
        let end = end.max(-1);
        let mut i = beg;
        while i > end && p(self.s.tokens[i as usize].c) {
            i -= 1;
        }
        i
    }

    /// `tokens.skipIf(p, beg, end)`: on from `beg` while `p` holds, up to `end`, which it stops at.
    fn skip_if(&self, p: impl Fn(C) -> bool, beg: i32, end: i32) -> i32 {
        if beg < 0 {
            return beg;
        }
        let end = end.min(self.s.tokens.len() as i32);
        let mut i = beg;
        while i < end && p(self.s.tokens[i as usize].c) {
            i += 1;
        }
        i
    }

    fn beg_offset(&self, (beg, _): (i32, i32)) -> u32 {
        self.s.tokens[beg as usize].start
    }
    fn end_offset(&self, (beg, end): (i32, i32)) -> u32 {
        if end > beg { self.s.tokens[end as usize - 1].end } else { self.s.tokens[beg as usize].start }
    }

    /// `atPosWithBody`: `n` over the tokens `[start_pos, end_pos]` less the trivia around them (a last comment of its
    /// own line kept), its comments captured from the tokens before and after it where no child holds them.
    pub(super) fn at_pos_with_body(&mut self, start_pos: i32, n: u32, end_pos: i32) -> u32 {
        let (mut start, mut end_excl) = (start_pos, start_pos);
        if end_pos >= start_pos {
            let non_space_end = self.rskip_if(C::whitespace, end_pos, start_pos - 1);
            if non_space_end < start_pos {
                end_excl = if end_pos == start_pos { end_pos } else { end_pos + 1 };
            } else {
                let first_non_trivia = self.skip_if(C::trivia, start_pos, end_pos + 1);
                if first_non_trivia > end_pos {
                    end_excl = non_space_end + 1;
                } else {
                    start = first_non_trivia;
                    end_excl = if self.s.tokens[non_space_end as usize].c != C::Comment {
                        non_space_end + 1
                    } else {
                        let end = self.rskip_if(C::htrivia, non_space_end - 1, first_non_trivia);
                        (if self.tc(end).is_some_and(C::at_eol_or_f) { non_space_end } else { end }) + 1
                    };
                }
            }
        }
        self.nodes[n as usize].origin = Some((start, end_excl));
        self.capture_comments(n, start, end_excl, end_pos);
        n
    }

    /// `atPosWithBody`'s capture of the comments a node's tokens leave out (`begComment`, `endComment`), unless the
    /// node has them already; a child's of its own tokens kept by it, a block's own (`Tree.Block`) by its children.
    fn capture_comments(&mut self, n: u32, start: i32, end_excl: i32, end_pos: i32) {
        let node = &self.nodes[n as usize];
        if node.beg_comment.is_some() || node.end_comment.is_some() {
            return;
        }
        let (mut min_child, mut min_beg, mut max_child, mut max_end) = (None, u32::MAX, None, 0i64 - 1);
        for &(_, c) in &node.fields {
            let x = &self.nodes[c as usize];
            let (Some(o), bc, ec) = (x.origin, x.beg_comment, x.end_comment) else { continue };
            let beg = self.beg_offset(bc.unwrap_or(o));
            let end = self.end_offset(ec.unwrap_or(o));
            if beg < end {
                if beg < min_beg {
                    min_child = Some(c);
                    min_beg = beg;
                }
                if (end as i64) > max_end {
                    max_child = Some(c);
                    max_end = end as i64;
                }
            }
        }
        let is_block = node.kind.tree_block();
        let is_term_block = node.kind == crate::fix::nodes::K::TermBlock;
        let beg_comment = if self.s.tokens[start.max(0) as usize].start < min_beg {
            let mut found: Vec<i32> = Vec::new();
            let (mut idx, mut pending, mut had_eol) = (start - 1, 0usize, false);
            loop {
                let Some(c) = self.tc(idx) else { break };
                let go = match c {
                    C::Comment => {
                        found.insert(0, idx);
                        pending += 1;
                        true
                    }
                    c if c.at_eol() => {
                        if had_eol && pending == 0 {
                            false
                        } else {
                            had_eol = true;
                            pending = 0;
                            true
                        }
                    }
                    c if c.hspace() => true,
                    c => {
                        if !found.is_empty() && (self.s.can_end_stat(c) || is_term_block || had_eol && c == C::Comma) {
                            found.drain(0..pending.min(found.len()));
                        }
                        let t = self.s.tokens[idx as usize];
                        t.start == t.end
                    }
                };
                if !go {
                    break;
                }
                idx -= 1;
            }
            (!found.is_empty()).then(|| (found[0], found[found.len() - 1] + 1))
        } else if is_block {
            None
        } else {
            min_child.and_then(|c| self.nodes[c as usize].beg_comment)
        };
        let end_braceless = is_block && self.skip_if(C::htrivia, end_excl, end_pos + 1) <= end_pos;
        let end_comment = if end_braceless {
            None
        } else if end_excl >= 1 && (self.s.tokens[end_excl as usize - 1].end as i64) > max_end {
            let mut found: Vec<i32> = Vec::new();
            let mut idx = end_excl;
            loop {
                let Some(c) = self.tc(idx) else { break };
                let go = match c {
                    C::Comment => {
                        found.push(idx);
                        true
                    }
                    c if c.hspace() => true,
                    C::Comma => {
                        let k = self.skip_if(C::htrivia, idx + 1, i32::MAX);
                        self.tc(k).is_some_and(C::at_eol)
                    }
                    _ => {
                        let t = self.s.tokens[idx as usize];
                        t.start == t.end
                    }
                };
                if !go {
                    break;
                }
                idx += 1;
            }
            (!found.is_empty()).then(|| (found[0], found[found.len() - 1] + 1))
        } else if is_block {
            None
        } else {
            max_child.and_then(|c| self.nodes[c as usize].end_comment)
        };
        let node = &mut self.nodes[n as usize];
        node.beg_comment = beg_comment;
        node.end_comment = end_comment;
    }

    /// `atPos(start, end)`: over the tokens `[start, end]`.
    pub(super) fn at_pos(&mut self, start: i32, end: i32, n: u32) -> u32 {
        self.at_pos_with_body(start, n, end)
    }
    /// `atPosOpt`: a node positioned already keeps its position.
    pub(super) fn at_pos_opt(&mut self, start: i32, end: i32, n: u32) -> u32 {
        if self.nodes[n as usize].origin.is_some() { n } else { self.at_pos(start, end, n) }
    }
    /// `atPos(pos)`: over the one token at `pos`.
    pub(super) fn at_pos1(&mut self, pos: i32, n: u32) -> u32 {
        self.at_pos_with_body(pos, n, pos)
    }
    /// `atCurPos`.
    pub(super) fn at_cur_pos(&mut self, n: u32) -> u32 {
        let pos = self.curr_index();
        self.at_pos1(pos, n)
    }
    /// `atCurPosNext`: at the current token, then past it.
    pub(super) fn at_cur_pos_next(&mut self, n: u32) -> u32 {
        let n = self.at_cur_pos(n);
        self.next();
        n
    }
    /// `atPosEmpty(pos)`: of no width, at the token at `pos`.
    pub(super) fn at_pos_empty(&mut self, pos: i32, n: u32) -> u32 {
        self.at_pos_with_body(pos, n, pos - 1)
    }
    /// `atCurPosEmpty`.
    pub(super) fn at_cur_pos_empty(&mut self, n: u32) -> u32 {
        let pos = self.curr_index();
        self.at_pos_empty(pos, n)
    }
    /// `autoPos`: from the current token to the last `body` consumed.
    pub(super) fn auto_pos(&mut self, body: impl FnOnce(&mut Self) -> P<u32>) -> P<u32> {
        let start = self.curr_index();
        let n = body(self)?;
        let end = self.prev_index();
        Ok(self.at_pos(start, end, n))
    }
    /// `autoPosOpt`.
    pub(super) fn auto_pos_opt(&mut self, body: impl FnOnce(&mut Self) -> P<u32>) -> P<u32> {
        let start = self.curr_index();
        let n = body(self)?;
        let end = self.prev_index();
        Ok(self.at_pos_opt(start, end, n))
    }
    /// `autoEndPos(start)`: from the token at `start` to the last `body` consumed.
    pub(super) fn auto_end_pos(&mut self, start: i32, body: impl FnOnce(&mut Self) -> P<u32>) -> P<u32> {
        let n = body(self)?;
        let end = self.prev_index();
        Ok(self.at_pos(start, end, n))
    }
    /// `autoEndPosOpt(start)`.
    pub(super) fn auto_end_pos_opt(&mut self, start: i32, body: impl FnOnce(&mut Self) -> P<u32>) -> P<u32> {
        let n = body(self)?;
        let end = self.prev_index();
        Ok(self.at_pos_opt(start, end, n))
    }
    /// `copyPos(from)(n)`: `from`'s position and comments.
    pub(super) fn copy_pos(&mut self, from: u32, n: u32) -> u32 {
        let (origin, bc, ec) = {
            let f = &self.nodes[from as usize];
            (f.origin, f.beg_comment, f.end_comment)
        };
        let node = &mut self.nodes[n as usize];
        node.origin = origin;
        node.beg_comment = bc;
        node.end_comment = ec;
        n
    }
    /// A positioned node's first and last token (`TreePos`: `begIndex`, `endIndex`).
    pub(super) fn beg_index(&self, n: u32) -> i32 {
        self.nodes[n as usize].origin.map_or(0, |o| o.0)
    }
    pub(super) fn end_index(&self, n: u32) -> i32 {
        self.nodes[n as usize].origin.map_or(0, |o| o.1 - 1)
    }

    /// `parseRule`: `rule` from past `BOF` to `EOF`, positioned over the whole text.
    pub(super) fn parse_rule(&mut self, rule: impl FnOnce(&mut Self) -> P<u32>) -> P<u32> {
        self.accept(C::Bof)?;
        let start = self.prev_index();
        let t = rule(self)?;
        let end = self.curr_index();
        self.accept(C::Eof)?;
        Ok(self.at_pos(start, end, t))
    }
}
