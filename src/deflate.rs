//! DEFLATE (RFC 1951) as zlib 1.3.1 writes it, byte for byte: the same window, hash chains, lazy
//! matching and block decisions (`deflate.c`), the same trees (`trees.c`), so that what
//! `java.util.zip.Deflater` gives under `teq interp` is what the JDK's bundled zlib gives (and
//! Python's, over the same zlib). One stream at a time, compressed whole at its end (`Z_FINISH`
//! from the start, which gives zlib's output for any input split under `Z_NO_FLUSH`); levels 1 to
//! 9 with the default strategy, level 0 as stored blocks; `wrap` adds zlib's header and Adler-32.

const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
const MIN_LOOKAHEAD: usize = MAX_MATCH + MIN_MATCH + 1;
const TOO_FAR: usize = 4096;
const W_BITS: usize = 15;
const W_SIZE: usize = 1 << W_BITS;
const W_MASK: usize = W_SIZE - 1;
const WINDOW_SIZE: usize = 2 * W_SIZE;
const MAX_DIST: usize = W_SIZE - MIN_LOOKAHEAD;
const MEM_LEVEL: usize = 8;
const HASH_BITS: usize = MEM_LEVEL + 7;
const HASH_SIZE: usize = 1 << HASH_BITS;
const HASH_MASK: usize = HASH_SIZE - 1;
const HASH_SHIFT: usize = HASH_BITS.div_ceil(MIN_MATCH);
const LIT_BUFSIZE: usize = 1 << (MEM_LEVEL + 6);
const SYM_END: usize = (LIT_BUFSIZE - 1) * 3;
/// zlib's `WIN_INIT`: the bytes past the data zeroed, which a match may compare against.
const WIN_INIT: usize = MAX_MATCH;
const NIL: usize = 0;

const LENGTH_CODES: usize = 29;
const LITERALS: usize = 256;
const L_CODES: usize = LITERALS + 1 + LENGTH_CODES;
const D_CODES: usize = 30;
const BL_CODES: usize = 19;
const HEAP_SIZE: usize = 2 * L_CODES + 1;
const MAX_BITS: usize = 15;
const MAX_BL_BITS: usize = 7;
const END_BLOCK: usize = 256;
const REP_3_6: usize = 16;
const REPZ_3_10: usize = 17;
const REPZ_11_138: usize = 18;

const EXTRA_LBITS: [u8; LENGTH_CODES] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const EXTRA_DBITS: [u8; D_CODES] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];
const EXTRA_BLBITS: [u8; BL_CODES] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 3, 7];
const BL_ORDER: [usize; BL_CODES] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

/// zlib's `configuration_table`: good_length, max_lazy (max_insert for the fast path), nice_length,
/// max_chain, and whether the lazy evaluation (`deflate_slow`) runs.
const CONFIG: [(usize, usize, usize, usize, bool); 10] = [
    (0, 0, 0, 0, false),
    (4, 4, 8, 4, false),
    (4, 5, 16, 8, false),
    (4, 6, 32, 32, false),
    (4, 4, 16, 16, true),
    (8, 16, 32, 32, true),
    (8, 16, 128, 128, true),
    (8, 32, 128, 256, true),
    (32, 128, 258, 1024, true),
    (32, 258, 258, 4096, true),
];

/// The tables `tr_static_init` makes.
struct Tables {
    length_code: [u8; 256],
    base_length: [u16; LENGTH_CODES],
    dist_code: [u8; 512],
    base_dist: [u16; D_CODES],
    static_ltree: [(u16, u16); L_CODES + 2],
    static_dtree: [(u16, u16); D_CODES],
}

fn tables() -> &'static Tables {
    static T: std::sync::OnceLock<Tables> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        let mut t = Tables { length_code: [0; 256], base_length: [0; LENGTH_CODES], dist_code: [0; 512], base_dist: [0; D_CODES], static_ltree: [(0, 0); L_CODES + 2], static_dtree: [(0, 0); D_CODES] };
        let mut length = 0usize;
        let mut code = 0usize;
        while code < LENGTH_CODES - 1 {
            t.base_length[code] = length as u16;
            for _ in 0..(1 << EXTRA_LBITS[code]) {
                t.length_code[length] = code as u8;
                length += 1;
            }
            code += 1;
        }
        t.length_code[length - 1] = code as u8;
        let mut dist = 0usize;
        code = 0;
        while code < 16 {
            t.base_dist[code] = dist as u16;
            for _ in 0..(1 << EXTRA_DBITS[code]) {
                t.dist_code[dist] = code as u8;
                dist += 1;
            }
            code += 1;
        }
        dist >>= 7;
        while code < D_CODES {
            t.base_dist[code] = (dist << 7) as u16;
            for _ in 0..(1 << (EXTRA_DBITS[code] - 7)) {
                t.dist_code[256 + dist] = code as u8;
                dist += 1;
            }
            code += 1;
        }
        let mut bl_count = [0u16; MAX_BITS + 1];
        let mut lens = [0u16; L_CODES + 2];
        for (n, l) in lens.iter_mut().enumerate() {
            *l = match n {
                0..=143 => 8,
                144..=255 => 9,
                256..=279 => 7,
                _ => 8,
            };
            bl_count[*l as usize] += 1;
        }
        let codes = gen_codes(&lens, L_CODES + 1, &bl_count);
        for n in 0..L_CODES + 2 {
            t.static_ltree[n] = (codes[n], lens[n]);
        }
        for n in 0..D_CODES {
            t.static_dtree[n] = (bi_reverse(n as u32, 5) as u16, 5);
        }
        t
    })
}

fn bi_reverse(code: u32, len: usize) -> u32 {
    code.reverse_bits() >> (32 - len)
}

/// The codes of a tree from its lengths, as `gen_codes` makes them, for the codes 0..=max_code.
fn gen_codes(lens: &[u16], max_code: usize, bl_count: &[u16; MAX_BITS + 1]) -> Vec<u16> {
    let mut next_code = [0u16; MAX_BITS + 1];
    let mut code = 0u32;
    for bits in 1..=MAX_BITS {
        code = (code + bl_count[bits - 1] as u32) << 1;
        next_code[bits] = code as u16;
    }
    let mut codes = vec![0u16; lens.len()];
    for n in 0..=max_code.min(lens.len() - 1) {
        let len = lens[n] as usize;
        if len == 0 {
            continue;
        }
        codes[n] = bi_reverse(next_code[len] as u32, len) as u16;
        next_code[len] += 1;
    }
    codes
}

fn d_code(t: &Tables, dist: usize) -> usize {
    if dist < 256 {
        t.dist_code[dist] as usize
    } else {
        t.dist_code[256 + (dist >> 7)] as usize
    }
}

/// A dynamic tree as `trees.c` keeps it: the frequency or code (one field), and the father or
/// length (one field), as zlib's unions overlay them.
struct Tree {
    fc: Vec<u16>,
    dl: Vec<u16>,
    max_code: usize,
    elems: usize,
    extra: &'static [u8],
    extra_base: usize,
    max_length: usize,
    /// The static tree's lengths, for `static_len`.
    stat: Option<Vec<u16>>,
}

impl Tree {
    fn new(elems: usize, extra: &'static [u8], extra_base: usize, max_length: usize, stat: Option<Vec<u16>>) -> Tree {
        Tree { fc: vec![0; HEAP_SIZE], dl: vec![0; HEAP_SIZE], max_code: 0, elems, extra, extra_base, max_length, stat }
    }
}

struct Out {
    bytes: Vec<u8>,
    bit_buf: u64,
    bit_count: u32,
}

impl Out {
    #[inline]
    fn send_bits(&mut self, value: u32, length: usize) {
        self.bit_buf |= (value as u64) << self.bit_count;
        self.bit_count += length as u32;
        while self.bit_count >= 8 {
            self.bytes.push(self.bit_buf as u8);
            self.bit_buf >>= 8;
            self.bit_count -= 8;
        }
    }

    fn windup(&mut self) {
        if self.bit_count > 0 {
            self.bytes.push(self.bit_buf as u8);
        }
        self.bit_buf = 0;
        self.bit_count = 0;
    }
}

struct State<'a> {
    input: &'a [u8],
    /// How much of the input the window has taken in.
    next_in: usize,
    window: Vec<u8>,
    high_water: usize,
    prev: Vec<u16>,
    head: Vec<u16>,
    ins_h: usize,
    strstart: usize,
    block_start: isize,
    lookahead: usize,
    insert: usize,
    match_start: usize,
    match_length: usize,
    prev_match: usize,
    prev_length: usize,
    match_available: bool,
    good_match: usize,
    max_lazy: usize,
    nice_match: usize,
    max_chain: usize,
    sym_buf: Vec<u8>,
    ltree: Tree,
    dtree: Tree,
    bltree: Tree,
    heap: [usize; HEAP_SIZE],
    heap_len: usize,
    heap_max: usize,
    depth: [u8; HEAP_SIZE],
    bl_count: [u16; MAX_BITS + 1],
    opt_len: u64,
    static_len: u64,
    out: Out,
}

/// The input compressed whole: raw DEFLATE, or with `wrap` zlib's stream (header and Adler-32).
pub fn deflate(input: &[u8], level: i32, wrap: bool) -> Vec<u8> {
    let level = if level < 0 { 6 } else { level.min(9) as usize };
    let mut out = Out { bytes: Vec::with_capacity(input.len() / 2 + 64), bit_buf: 0, bit_count: 0 };
    if wrap {
        let flags = if level < 2 { 0 } else if level < 6 { 1 } else if level == 6 { 2 } else { 3 };
        let mut header: u32 = ((8 + ((W_BITS as u32 - 8) << 4)) << 8) | (flags << 6);
        header += 31 - (header % 31);
        out.bytes.extend_from_slice(&(header as u16).to_be_bytes());
    }
    let t = tables();
    let mut s = State {
        input,
        next_in: 0,
        window: vec![0; WINDOW_SIZE],
        high_water: 0,
        prev: vec![0; W_SIZE],
        head: vec![0; HASH_SIZE],
        ins_h: 0,
        strstart: 0,
        block_start: 0,
        lookahead: 0,
        insert: 0,
        match_start: 0,
        match_length: MIN_MATCH - 1,
        prev_match: 0,
        prev_length: MIN_MATCH - 1,
        match_available: false,
        good_match: CONFIG[level].0,
        max_lazy: CONFIG[level].1,
        nice_match: CONFIG[level].2,
        max_chain: CONFIG[level].3,
        sym_buf: Vec::with_capacity(SYM_END),
        ltree: Tree::new(L_CODES, &EXTRA_LBITS, LITERALS + 1, MAX_BITS, Some(t.static_ltree.iter().map(|&(_, l)| l).collect())),
        dtree: Tree::new(D_CODES, &EXTRA_DBITS, 0, MAX_BITS, Some(t.static_dtree.iter().map(|&(_, l)| l).collect())),
        bltree: Tree::new(BL_CODES, &EXTRA_BLBITS, 0, MAX_BL_BITS, None),
        heap: [0; HEAP_SIZE],
        heap_len: 0,
        heap_max: 0,
        depth: [0; HEAP_SIZE],
        bl_count: [0; MAX_BITS + 1],
        opt_len: 0,
        static_len: 0,
        out,
    };
    s.init_block();
    if level == 0 {
        s.stored();
    } else if CONFIG[level].4 {
        s.deflate_slow();
    } else {
        s.deflate_fast();
    }
    let mut out = s.out;
    if wrap {
        out.bytes.extend_from_slice(&adler32(input).to_be_bytes());
    }
    out.bytes
}

pub fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in data.chunks(5552) {
        for &x in chunk {
            a += x as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

impl State<'_> {
    // ---- deflate.c ----

    #[inline]
    fn update_hash(&mut self, c: u8) {
        self.ins_h = ((self.ins_h << HASH_SHIFT) ^ c as usize) & HASH_MASK;
    }

    /// `INSERT_STRING`: the string at `at` entered in its hash chain; the chain's former head.
    #[inline]
    fn insert_string(&mut self, at: usize) -> usize {
        self.update_hash(self.window[at + MIN_MATCH - 1]);
        let head = self.head[self.ins_h] as usize;
        self.prev[at & W_MASK] = head as u16;
        self.head[self.ins_h] = at as u16;
        head
    }

    fn slide_hash(&mut self) {
        for h in self.head.iter_mut() {
            *h = if *h as usize >= W_SIZE { *h - W_SIZE as u16 } else { NIL as u16 };
        }
        for p in self.prev.iter_mut() {
            *p = if *p as usize >= W_SIZE { *p - W_SIZE as u16 } else { NIL as u16 };
        }
    }

    fn fill_window(&mut self) {
        loop {
            let mut more = WINDOW_SIZE - self.lookahead - self.strstart;
            if self.strstart >= W_SIZE + MAX_DIST {
                self.window.copy_within(W_SIZE..W_SIZE + W_SIZE - more, 0);
                self.match_start = self.match_start.wrapping_sub(W_SIZE);
                self.strstart -= W_SIZE;
                self.block_start -= W_SIZE as isize;
                if self.insert > self.strstart {
                    self.insert = self.strstart;
                }
                self.slide_hash();
                more += W_SIZE;
            }
            if self.next_in == self.input.len() {
                break;
            }
            let n = more.min(self.input.len() - self.next_in);
            let at = self.strstart + self.lookahead;
            self.window[at..at + n].copy_from_slice(&self.input[self.next_in..self.next_in + n]);
            self.next_in += n;
            self.lookahead += n;
            if self.lookahead + self.insert >= MIN_MATCH {
                let mut str = self.strstart - self.insert;
                self.ins_h = self.window[str] as usize;
                self.update_hash(self.window[str + 1]);
                while self.insert > 0 {
                    self.update_hash(self.window[str + MIN_MATCH - 1]);
                    self.prev[str & W_MASK] = self.head[self.ins_h];
                    self.head[self.ins_h] = str as u16;
                    str += 1;
                    self.insert -= 1;
                    if self.lookahead + self.insert < MIN_MATCH {
                        break;
                    }
                }
            }
            if !(self.lookahead < MIN_LOOKAHEAD && self.next_in < self.input.len()) {
                break;
            }
        }
        // zlib zeroes the bytes a match may read past the data (`high_water`); the window here is
        // zeroed from the start, so that only what zlib would leave there is there.
        if self.high_water < WINDOW_SIZE {
            let curr = self.strstart + self.lookahead;
            if self.high_water < curr {
                let init = (WINDOW_SIZE - curr).min(WIN_INIT);
                self.window[curr..curr + init].fill(0);
                self.high_water = curr + init;
            } else if self.high_water < curr + WIN_INIT {
                let init = (curr + WIN_INIT - self.high_water).min(WINDOW_SIZE - self.high_water);
                self.window[self.high_water..self.high_water + init].fill(0);
                self.high_water += init;
            }
        }
    }

    fn longest_match(&mut self, mut cur_match: usize) -> usize {
        let mut chain_length = self.max_chain;
        let scan = self.strstart;
        let mut best_len = self.prev_length;
        let mut nice_match = self.nice_match;
        let limit = if self.strstart > MAX_DIST { self.strstart - MAX_DIST } else { NIL };
        let strend = self.strstart + MAX_MATCH;
        let w = &self.window;
        let mut scan_end1 = w[scan + best_len - 1];
        let mut scan_end = w[scan + best_len];
        if self.prev_length >= self.good_match {
            chain_length >>= 2;
        }
        if nice_match > self.lookahead {
            nice_match = self.lookahead;
        }
        loop {
            let m = cur_match;
            if !(w[m + best_len] != scan_end || w[m + best_len - 1] != scan_end1 || w[m] != w[scan] || w[m + 1] != w[scan + 1]) {
                // Equal for the first two bytes, compared from the third to `strend`.
                let mut s = scan + 2;
                let mut mm = m + 2;
                while s < strend && w[s] == w[mm] {
                    s += 1;
                    mm += 1;
                }
                let len = MAX_MATCH - (strend - s);
                if len > best_len {
                    self.match_start = cur_match;
                    best_len = len;
                    if len >= nice_match {
                        break;
                    }
                    scan_end1 = w[scan + best_len - 1];
                    scan_end = w[scan + best_len];
                }
            }
            cur_match = self.prev[cur_match & W_MASK] as usize;
            if cur_match <= limit {
                break;
            }
            chain_length -= 1;
            if chain_length == 0 {
                break;
            }
        }
        best_len.min(self.lookahead)
    }

    fn flush_block(&mut self, last: bool) {
        let stored_len = (self.strstart as isize - self.block_start) as usize;
        let buf = (self.block_start >= 0).then_some(self.block_start as usize);
        self.tr_flush_block(buf, stored_len, last);
        self.block_start = self.strstart as isize;
    }

    fn deflate_fast(&mut self) {
        loop {
            if self.lookahead < MIN_LOOKAHEAD {
                self.fill_window();
                if self.lookahead == 0 {
                    break;
                }
            }
            let mut hash_head = NIL;
            if self.lookahead >= MIN_MATCH {
                hash_head = self.insert_string(self.strstart);
            }
            if hash_head != NIL && self.strstart - hash_head <= MAX_DIST {
                self.match_length = self.longest_match(hash_head);
            }
            let bflush;
            if self.match_length >= MIN_MATCH {
                bflush = self.tally_dist(self.strstart - self.match_start, self.match_length - MIN_MATCH);
                self.lookahead -= self.match_length;
                if self.match_length <= self.max_lazy && self.lookahead >= MIN_MATCH {
                    self.match_length -= 1;
                    loop {
                        self.strstart += 1;
                        self.insert_string(self.strstart);
                        self.match_length -= 1;
                        if self.match_length == 0 {
                            break;
                        }
                    }
                    self.strstart += 1;
                } else {
                    self.strstart += self.match_length;
                    self.match_length = 0;
                    self.ins_h = self.window[self.strstart] as usize;
                    self.update_hash(self.window[self.strstart + 1]);
                }
            } else {
                bflush = self.tally_lit(self.window[self.strstart]);
                self.lookahead -= 1;
                self.strstart += 1;
            }
            if bflush {
                self.flush_block(false);
            }
        }
        self.insert = self.strstart.min(MIN_MATCH - 1);
        self.flush_block(true);
    }

    fn deflate_slow(&mut self) {
        loop {
            if self.lookahead < MIN_LOOKAHEAD {
                self.fill_window();
                if self.lookahead == 0 {
                    break;
                }
            }
            let mut hash_head = NIL;
            if self.lookahead >= MIN_MATCH {
                hash_head = self.insert_string(self.strstart);
            }
            self.prev_length = self.match_length;
            self.prev_match = self.match_start;
            self.match_length = MIN_MATCH - 1;
            if hash_head != NIL && self.prev_length < self.max_lazy && self.strstart - hash_head <= MAX_DIST {
                self.match_length = self.longest_match(hash_head);
                if self.match_length <= 5 && self.match_length == MIN_MATCH && self.strstart - self.match_start > TOO_FAR {
                    self.match_length = MIN_MATCH - 1;
                }
            }
            if self.prev_length >= MIN_MATCH && self.match_length <= self.prev_length {
                let max_insert = self.strstart + self.lookahead - MIN_MATCH;
                let bflush = self.tally_dist(self.strstart - 1 - self.prev_match, self.prev_length - MIN_MATCH);
                self.lookahead -= self.prev_length - 1;
                self.prev_length -= 2;
                loop {
                    self.strstart += 1;
                    if self.strstart <= max_insert {
                        self.insert_string(self.strstart);
                    }
                    self.prev_length -= 1;
                    if self.prev_length == 0 {
                        break;
                    }
                }
                self.match_available = false;
                self.match_length = MIN_MATCH - 1;
                self.strstart += 1;
                if bflush {
                    self.flush_block(false);
                }
            } else if self.match_available {
                let bflush = self.tally_lit(self.window[self.strstart - 1]);
                if bflush {
                    self.flush_block(false);
                }
                self.strstart += 1;
                self.lookahead -= 1;
            } else {
                self.match_available = true;
                self.strstart += 1;
                self.lookahead -= 1;
            }
        }
        if self.match_available {
            self.tally_lit(self.window[self.strstart - 1]);
            self.match_available = false;
        }
        self.insert = self.strstart.min(MIN_MATCH - 1);
        self.flush_block(true);
    }

    /// Level 0: the input in stored blocks of at most 65535 bytes, the last marked.
    fn stored(&mut self) {
        let input = self.input;
        let mut chunks = input.chunks(65535).peekable();
        if chunks.peek().is_none() {
            self.stored_block(&[], true);
        }
        while let Some(chunk) = chunks.next() {
            let last = chunks.peek().is_none();
            self.stored_block(chunk, last);
        }
    }

    // ---- trees.c ----

    fn init_block(&mut self) {
        for n in 0..L_CODES {
            self.ltree.fc[n] = 0;
        }
        for n in 0..D_CODES {
            self.dtree.fc[n] = 0;
        }
        for n in 0..BL_CODES {
            self.bltree.fc[n] = 0;
        }
        self.ltree.fc[END_BLOCK] = 1;
        self.opt_len = 0;
        self.static_len = 0;
        self.sym_buf.clear();
    }

    #[inline]
    fn tally_lit(&mut self, c: u8) -> bool {
        self.sym_buf.extend_from_slice(&[0, 0, c]);
        self.ltree.fc[c as usize] += 1;
        self.sym_buf.len() == SYM_END
    }

    #[inline]
    fn tally_dist(&mut self, dist: usize, len: usize) -> bool {
        let t = tables();
        self.sym_buf.extend_from_slice(&[dist as u8, (dist >> 8) as u8, len as u8]);
        self.ltree.fc[t.length_code[len] as usize + LITERALS + 1] += 1;
        self.dtree.fc[d_code(t, dist - 1)] += 1;
        self.sym_buf.len() == SYM_END
    }

    fn tree(&mut self, which: u8) -> &mut Tree {
        match which {
            0 => &mut self.ltree,
            1 => &mut self.dtree,
            _ => &mut self.bltree,
        }
    }

    #[inline]
    fn smaller(tree: &Tree, depth: &[u8; HEAP_SIZE], n: usize, m: usize) -> bool {
        tree.fc[n] < tree.fc[m] || (tree.fc[n] == tree.fc[m] && depth[n] <= depth[m])
    }

    fn pqdownheap(&mut self, which: u8, mut k: usize) {
        let v = self.heap[k];
        let mut j = k << 1;
        while j <= self.heap_len {
            let tree = match which {
                0 => &self.ltree,
                1 => &self.dtree,
                _ => &self.bltree,
            };
            if j < self.heap_len && Self::smaller(tree, &self.depth, self.heap[j + 1], self.heap[j]) {
                j += 1;
            }
            if Self::smaller(tree, &self.depth, v, self.heap[j]) {
                break;
            }
            self.heap[k] = self.heap[j];
            k = j;
            j <<= 1;
        }
        self.heap[k] = v;
    }

    fn build_tree(&mut self, which: u8) {
        let elems = self.tree(which).elems;
        let mut max_code: isize = -1;
        self.heap_len = 0;
        self.heap_max = HEAP_SIZE;
        for n in 0..elems {
            if self.tree(which).fc[n] != 0 {
                self.heap_len += 1;
                self.heap[self.heap_len] = n;
                max_code = n as isize;
                self.depth[n] = 0;
            } else {
                self.tree(which).dl[n] = 0;
            }
        }
        while self.heap_len < 2 {
            let node = if max_code < 2 {
                max_code += 1;
                max_code as usize
            } else {
                0
            };
            self.heap_len += 1;
            self.heap[self.heap_len] = node;
            self.tree(which).fc[node] = 1;
            self.depth[node] = 0;
            self.opt_len = self.opt_len.wrapping_sub(1);
            let stat = self.tree(which).stat.as_ref().map(|s| s[node] as u64);
            if let Some(l) = stat {
                self.static_len = self.static_len.wrapping_sub(l);
            }
        }
        self.tree(which).max_code = max_code as usize;
        let mut n = self.heap_len / 2;
        while n >= 1 {
            self.pqdownheap(which, n);
            n -= 1;
        }
        let mut node = elems;
        loop {
            // pqremove
            let n = self.heap[1];
            self.heap[1] = self.heap[self.heap_len];
            self.heap_len -= 1;
            self.pqdownheap(which, 1);
            let m = self.heap[1];
            self.heap_max -= 1;
            self.heap[self.heap_max] = n;
            self.heap_max -= 1;
            self.heap[self.heap_max] = m;
            let tree = self.tree(which);
            tree.fc[node] = tree.fc[n].wrapping_add(tree.fc[m]);
            tree.dl[n] = node as u16;
            tree.dl[m] = node as u16;
            self.depth[node] = self.depth[n].max(self.depth[m]) + 1;
            self.heap[1] = node;
            node += 1;
            self.pqdownheap(which, 1);
            if self.heap_len < 2 {
                break;
            }
        }
        self.heap_max -= 1;
        self.heap[self.heap_max] = self.heap[1];
        self.gen_bitlen(which);
        let max_code = self.tree(which).max_code;
        let bl_count = self.bl_count;
        let tree = self.tree(which);
        let codes = gen_codes(&tree.dl, max_code, &bl_count);
        for n in 0..=max_code {
            if tree.dl[n] != 0 {
                tree.fc[n] = codes[n];
            }
        }
    }

    fn gen_bitlen(&mut self, which: u8) {
        self.bl_count = [0; MAX_BITS + 1];
        let root = self.heap[self.heap_max];
        let (max_code, max_length, extra_base) = {
            let t = self.tree(which);
            (t.max_code, t.max_length, t.extra_base)
        };
        self.tree(which).dl[root] = 0;
        let mut overflow = 0i32;
        let mut h = self.heap_max + 1;
        while h < HEAP_SIZE {
            let n = self.heap[h];
            let tree = match which {
                0 => &mut self.ltree,
                1 => &mut self.dtree,
                _ => &mut self.bltree,
            };
            let mut bits = tree.dl[tree.dl[n] as usize] as usize + 1;
            if bits > max_length {
                bits = max_length;
                overflow += 1;
            }
            tree.dl[n] = bits as u16;
            h += 1;
            if n > max_code {
                continue;
            }
            self.bl_count[bits] += 1;
            let xbits = if n >= extra_base { tree.extra[n - extra_base] as u64 } else { 0 };
            let f = tree.fc[n] as u64;
            self.opt_len = self.opt_len.wrapping_add(f * (bits as u64 + xbits));
            if let Some(stat) = &tree.stat {
                self.static_len = self.static_len.wrapping_add(f * (stat[n] as u64 + xbits));
            }
        }
        if overflow == 0 {
            return;
        }
        loop {
            let mut bits = max_length - 1;
            while self.bl_count[bits] == 0 {
                bits -= 1;
            }
            self.bl_count[bits] -= 1;
            self.bl_count[bits + 1] += 2;
            self.bl_count[max_length] -= 1;
            overflow -= 2;
            if overflow <= 0 {
                break;
            }
        }
        let mut h = HEAP_SIZE;
        let mut bits = max_length;
        while bits != 0 {
            let mut n = self.bl_count[bits];
            while n != 0 {
                h -= 1;
                let m = self.heap[h];
                if m > max_code {
                    continue;
                }
                let tree = match which {
                    0 => &mut self.ltree,
                    1 => &mut self.dtree,
                    _ => &mut self.bltree,
                };
                if tree.dl[m] as usize != bits {
                    self.opt_len = self.opt_len.wrapping_add((bits as u64).wrapping_sub(tree.dl[m] as u64).wrapping_mul(tree.fc[m] as u64));
                    tree.dl[m] = bits as u16;
                }
                n -= 1;
            }
            bits -= 1;
        }
    }

    fn scan_tree(&mut self, which: u8) {
        let max_code = self.tree(which).max_code;
        let mut prevlen: isize = -1;
        let mut nextlen = self.tree(which).dl[0] as isize;
        let mut count = 0usize;
        let (mut max_count, mut min_count) = (7usize, 4usize);
        if nextlen == 0 {
            max_count = 138;
            min_count = 3;
        }
        self.tree(which).dl[max_code + 1] = 0xffff;
        for n in 0..=max_code {
            let curlen = nextlen;
            nextlen = self.tree(which).dl[n + 1] as isize;
            count += 1;
            if count < max_count && curlen == nextlen {
                continue;
            } else if count < min_count {
                self.bltree.fc[curlen as usize] += count as u16;
            } else if curlen != 0 {
                if curlen != prevlen {
                    self.bltree.fc[curlen as usize] += 1;
                }
                self.bltree.fc[REP_3_6] += 1;
            } else if count <= 10 {
                self.bltree.fc[REPZ_3_10] += 1;
            } else {
                self.bltree.fc[REPZ_11_138] += 1;
            }
            count = 0;
            prevlen = curlen;
            if nextlen == 0 {
                max_count = 138;
                min_count = 3;
            } else if curlen == nextlen {
                max_count = 6;
                min_count = 3;
            } else {
                max_count = 7;
                min_count = 4;
            }
        }
    }

    fn send_code(&mut self, c: usize, which: u8) {
        let (code, len) = {
            let t = self.tree(which);
            (t.fc[c], t.dl[c])
        };
        self.out.send_bits(code as u32, len as usize);
    }

    fn send_tree(&mut self, which: u8, max_code: usize) {
        let mut prevlen: isize = -1;
        let mut nextlen = self.tree(which).dl[0] as isize;
        let mut count = 0usize;
        let (mut max_count, mut min_count) = (7usize, 4usize);
        if nextlen == 0 {
            max_count = 138;
            min_count = 3;
        }
        for n in 0..=max_code {
            let curlen = nextlen;
            nextlen = self.tree(which).dl[n + 1] as isize;
            count += 1;
            if count < max_count && curlen == nextlen {
                continue;
            } else if count < min_count {
                while count != 0 {
                    self.send_code(curlen as usize, 2);
                    count -= 1;
                }
            } else if curlen != 0 {
                if curlen != prevlen {
                    self.send_code(curlen as usize, 2);
                    count -= 1;
                }
                self.send_code(REP_3_6, 2);
                self.out.send_bits(count as u32 - 3, 2);
            } else if count <= 10 {
                self.send_code(REPZ_3_10, 2);
                self.out.send_bits(count as u32 - 3, 3);
            } else {
                self.send_code(REPZ_11_138, 2);
                self.out.send_bits(count as u32 - 11, 7);
            }
            count = 0;
            prevlen = curlen;
            if nextlen == 0 {
                max_count = 138;
                min_count = 3;
            } else if curlen == nextlen {
                max_count = 6;
                min_count = 3;
            } else {
                max_count = 7;
                min_count = 4;
            }
        }
    }

    fn build_bl_tree(&mut self) -> usize {
        self.scan_tree(0);
        self.scan_tree(1);
        self.build_tree(2);
        let mut max_blindex = BL_CODES - 1;
        while max_blindex >= 3 {
            if self.bltree.dl[BL_ORDER[max_blindex]] != 0 {
                break;
            }
            max_blindex -= 1;
        }
        self.opt_len = self.opt_len.wrapping_add(3 * (max_blindex as u64 + 1) + 5 + 5 + 4);
        max_blindex
    }

    fn send_all_trees(&mut self, lcodes: usize, dcodes: usize, blcodes: usize) {
        self.out.send_bits(lcodes as u32 - 257, 5);
        self.out.send_bits(dcodes as u32 - 1, 5);
        self.out.send_bits(blcodes as u32 - 4, 4);
        for &order in BL_ORDER.iter().take(blcodes) {
            let len = self.bltree.dl[order];
            self.out.send_bits(len as u32, 3);
        }
        self.send_tree(0, lcodes - 1);
        self.send_tree(1, dcodes - 1);
    }

    /// The block's symbols with the trees given (`static_trees`: zlib's fixed ones).
    fn compress_block(&mut self, static_trees: bool) {
        let t = tables();
        let syms = std::mem::take(&mut self.sym_buf);
        let code_of = |s: &State, which: u8, c: usize| -> (u32, usize) {
            if static_trees {
                let (code, len) = if which == 0 { t.static_ltree[c] } else { t.static_dtree[c] };
                (code as u32, len as usize)
            } else {
                let tr = if which == 0 { &s.ltree } else { &s.dtree };
                (tr.fc[c] as u32, tr.dl[c] as usize)
            }
        };
        for sym in syms.chunks_exact(3) {
            let mut dist = sym[0] as usize | ((sym[1] as usize) << 8);
            let lc = sym[2] as usize;
            if dist == 0 {
                let (c, l) = code_of(self, 0, lc);
                self.out.send_bits(c, l);
            } else {
                let code = t.length_code[lc] as usize;
                let (c, l) = code_of(self, 0, code + LITERALS + 1);
                self.out.send_bits(c, l);
                let extra = EXTRA_LBITS[code] as usize;
                if extra != 0 {
                    self.out.send_bits((lc - t.base_length[code] as usize) as u32, extra);
                }
                dist -= 1;
                let code = d_code(t, dist);
                let (c, l) = code_of(self, 1, code);
                self.out.send_bits(c, l);
                let extra = EXTRA_DBITS[code] as usize;
                if extra != 0 {
                    self.out.send_bits((dist - t.base_dist[code] as usize) as u32, extra);
                }
            }
        }
        let (c, l) = code_of(self, 0, END_BLOCK);
        self.out.send_bits(c, l);
        self.sym_buf = syms;
    }

    fn stored_block(&mut self, data: &[u8], last: bool) {
        self.out.send_bits(last as u32, 3);
        self.out.windup();
        let len = data.len() as u16;
        self.out.bytes.extend_from_slice(&len.to_le_bytes());
        self.out.bytes.extend_from_slice(&(!len).to_le_bytes());
        self.out.bytes.extend_from_slice(data);
    }

    fn tr_flush_block(&mut self, buf: Option<usize>, stored_len: usize, last: bool) {
        self.build_tree(0);
        self.build_tree(1);
        let max_blindex = self.build_bl_tree();
        let mut opt_lenb = (self.opt_len.wrapping_add(3 + 7)) >> 3;
        let static_lenb = (self.static_len.wrapping_add(3 + 7)) >> 3;
        if static_lenb <= opt_lenb {
            opt_lenb = static_lenb;
        }
        if stored_len as u64 + 4 <= opt_lenb && buf.is_some() {
            let start = buf.unwrap();
            let data = self.window[start..start + stored_len].to_vec();
            self.stored_block(&data, last);
        } else if static_lenb == opt_lenb {
            self.out.send_bits(2 + last as u32, 3);
            self.compress_block(true);
        } else {
            self.out.send_bits(4 + last as u32, 3);
            let (l, d) = (self.ltree.max_code + 1, self.dtree.max_code + 1);
            self.send_all_trees(l, d, max_blindex + 1);
            self.compress_block(false);
        }
        self.init_block();
        if last {
            self.out.windup();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_the_inflater() {
        let mut data = Vec::new();
        for i in 0..200_000u32 {
            data.extend_from_slice(format!("line {} {}\n", i % 977, i * 7 % 13).as_bytes());
        }
        for level in [1, 2, 3, 4, 5, 6, 9] {
            let packed = deflate(&data, level, false);
            assert_eq!(crate::zip::inflate(&packed, data.len()).unwrap(), data, "level {level}");
        }
        assert_eq!(deflate(&[], 6, false), vec![3, 0]);
    }
}

