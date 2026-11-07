// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! The crossword itself: a puzzle read from Across Lite's `.puz` or from ipuz, its numbering and
//! words, and a solve in progress (entries, pencil marks, checks and reveals, the cursor, and the
//! clock). Pure logic, no UI: everything here is host-testable with `cargo test`.

use serde::{Deserialize, Serialize};

/// Which way a word runs.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Dir {
    #[default]
    Across,
    Down,
}

impl Dir {
    pub fn flip(self) -> Dir {
        match self {
            Dir::Across => Dir::Down,
            Dir::Down => Dir::Across,
        }
    }
}

/// One clue and the squares its answer fills, in reading order.
#[derive(Clone, Debug, PartialEq)]
pub struct Clue {
    pub number: u16,
    pub dir: Dir,
    pub text: String,
    pub cells: Vec<usize>,
}

/// A puzzle as published: the grid's solution, its numbering, and its clues. Square index =
/// row * width + col.
#[derive(Clone, Debug, PartialEq)]
pub struct Puzzle {
    pub width: usize,
    pub height: usize,
    /// Each square's answer, upper case: one letter, or several in a rebus square. Empty for a
    /// block.
    pub solution: Vec<String>,
    /// Squares drawn with a circle (a theme's hidden word, most often).
    pub circled: Vec<bool>,
    /// Each square's clue number, 0 where none starts.
    pub numbers: Vec<u16>,
    /// The Across clues by number, then the Down clues by number.
    pub clues: Vec<Clue>,
    pub title: String,
    pub author: String,
    pub copyright: String,
    pub notes: String,
    /// The Across and Down clue index each square belongs to.
    across_of: Vec<Option<usize>>,
    down_of: Vec<Option<usize>>,
}

/// Why a file could not be read as a puzzle.
#[derive(Debug, PartialEq)]
pub enum ParseError {
    NotAPuzzle,
    Truncated,
    /// The publisher locked the answers, so nothing could check or reveal them.
    Scrambled,
    BadGrid,
}

impl Puzzle {
    /// Build a puzzle from its solution grid (empty strings are blocks) and its clue texts by
    /// number, numbering the grid the standard way: a square starts a word where the square
    /// before it is a block or the edge, and the word runs at least two squares.
    pub fn new(
        width: usize,
        height: usize,
        solution: Vec<String>,
        circled: Vec<bool>,
        across: &[(u16, String)],
        down: &[(u16, String)],
    ) -> Result<Puzzle, ParseError> {
        if width == 0 || height == 0 || solution.len() != width * height {
            return Err(ParseError::BadGrid);
        }
        let open = |r: usize, c: usize| !solution[r * width + c].is_empty();
        let mut numbers = vec![0u16; width * height];
        let mut starts = Vec::new();
        let mut n = 0u16;
        for r in 0..height {
            for c in 0..width {
                if !open(r, c) {
                    continue;
                }
                let a = (c == 0 || !open(r, c - 1)) && c + 1 < width && open(r, c + 1);
                let d = (r == 0 || !open(r - 1, c)) && r + 1 < height && open(r + 1, c);
                if a || d {
                    n += 1;
                    numbers[r * width + c] = n;
                    starts.push((n, r, c, a, d));
                }
            }
        }
        let text = |list: &[(u16, String)], number: u16| {
            list.iter()
                .find(|(k, _)| *k == number)
                .map(|(_, t)| t.clone())
                .unwrap_or_default()
        };
        let mut clues = Vec::new();
        for &(number, r, c, a, _) in &starts {
            if a {
                let cells = (c..width)
                    .take_while(|&cc| open(r, cc))
                    .map(|cc| r * width + cc)
                    .collect();
                clues.push(Clue {
                    number,
                    dir: Dir::Across,
                    text: text(across, number),
                    cells,
                });
            }
        }
        for &(number, r, c, _, d) in &starts {
            if d {
                let cells = (r..height)
                    .take_while(|&rr| open(rr, c))
                    .map(|rr| rr * width + c)
                    .collect();
                clues.push(Clue {
                    number,
                    dir: Dir::Down,
                    text: text(down, number),
                    cells,
                });
            }
        }
        let mut across_of = vec![None; width * height];
        let mut down_of = vec![None; width * height];
        for (k, clue) in clues.iter().enumerate() {
            for &i in &clue.cells {
                match clue.dir {
                    Dir::Across => across_of[i] = Some(k),
                    Dir::Down => down_of[i] = Some(k),
                }
            }
        }
        Ok(Puzzle {
            width,
            height,
            circled: if circled.len() == solution.len() {
                circled
            } else {
                vec![false; solution.len()]
            },
            solution,
            numbers,
            clues,
            title: String::new(),
            author: String::new(),
            copyright: String::new(),
            notes: String::new(),
            across_of,
            down_of,
        })
    }

    pub fn is_block(&self, i: usize) -> bool {
        self.solution[i].is_empty()
    }

    /// The clue that `i` belongs to running `dir`, if a word of two or more squares does.
    pub fn clue_at(&self, i: usize, dir: Dir) -> Option<usize> {
        match dir {
            Dir::Across => self.across_of.get(i).copied().flatten(),
            Dir::Down => self.down_of.get(i).copied().flatten(),
        }
    }

    /// The open squares a solver fills in.
    pub fn open_count(&self) -> usize {
        self.solution.iter().filter(|s| !s.is_empty()).count()
    }

    /// Read Across Lite's `.puz`, the format most daily sources publish: a fixed header, the
    /// solution and fill grids, NUL-terminated strings (title, author, copyright, the clues in
    /// numbering order, notes), then optional sections for circles (GEXT) and rebus squares
    /// (GRBS and RTBL). Version 2 files are UTF-8; older ones Latin-1.
    pub fn from_puz(bytes: &[u8]) -> Result<Puzzle, ParseError> {
        const HEADER: usize = 0x34;
        if bytes.len() < HEADER || &bytes[0x02..0x0E] != b"ACROSS&DOWN\0" {
            return Err(ParseError::NotAPuzzle);
        }
        let width = bytes[0x2C] as usize;
        let height = bytes[0x2D] as usize;
        let count = u16::from_le_bytes([bytes[0x2E], bytes[0x2F]]) as usize;
        if u16::from_le_bytes([bytes[0x32], bytes[0x33]]) != 0 {
            return Err(ParseError::Scrambled);
        }
        let utf8 = bytes[0x18] >= b'2';
        let cells = width * height;
        let grid_end = HEADER + 2 * cells;
        if bytes.len() < grid_end {
            return Err(ParseError::Truncated);
        }
        let grid = &bytes[HEADER..HEADER + cells];
        let mut at = grid_end;
        let mut next_string = || -> Result<String, ParseError> {
            let end = bytes[at..]
                .iter()
                .position(|&b| b == 0)
                .ok_or(ParseError::Truncated)?;
            let raw = &bytes[at..at + end];
            at += end + 1;
            Ok(if utf8 {
                String::from_utf8_lossy(raw).into_owned()
            } else {
                raw.iter().map(|&b| b as char).collect()
            })
        };
        let title = next_string()?;
        let author = next_string()?;
        let copyright = next_string()?;
        let mut texts = Vec::with_capacity(count);
        for _ in 0..count {
            texts.push(next_string()?);
        }
        // Notes are optional in older files.
        let notes = next_string().unwrap_or_default();
        drop(next_string);

        let mut solution: Vec<String> = grid
            .iter()
            .map(|&b| match b {
                b'.' | b':' => String::new(),
                b => (b as char).to_ascii_uppercase().to_string(),
            })
            .collect();
        let mut circled = vec![false; cells];
        let mut rebus_keys: Option<Vec<u8>> = None;
        let mut rebus_words: Vec<(u8, String)> = Vec::new();
        // The extra sections: a four-letter name, a length, a checksum, the data and a NUL.
        while at + 8 <= bytes.len() {
            let name = &bytes[at..at + 4];
            let len = u16::from_le_bytes([bytes[at + 4], bytes[at + 5]]) as usize;
            let start = at + 8;
            if start + len > bytes.len() {
                break;
            }
            let data = &bytes[start..start + len];
            match name {
                b"GEXT" if len == cells => {
                    for (i, &f) in data.iter().enumerate() {
                        circled[i] = f & 0x80 != 0;
                    }
                }
                b"GRBS" if len == cells => rebus_keys = Some(data.to_vec()),
                b"RTBL" => {
                    for entry in String::from_utf8_lossy(data).split(';') {
                        if let Some((k, word)) = entry.split_once(':')
                            && let Ok(k) = k.trim().parse::<u8>()
                        {
                            rebus_words.push((k, word.trim().to_ascii_uppercase()));
                        }
                    }
                }
                _ => {}
            }
            at = start + len + 1;
        }
        if let Some(keys) = rebus_keys {
            for (i, &k) in keys.iter().enumerate() {
                if k == 0 {
                    continue;
                }
                if let Some((_, word)) = rebus_words.iter().find(|(key, _)| *key + 1 == k) {
                    solution[i] = word.clone();
                }
            }
        }

        // The clues are listed in numbering order, an Across before a Down at the same number.
        let shape = Puzzle::new(width, height, solution.clone(), Vec::new(), &[], &[])?;
        let mut order: Vec<&Clue> = shape.clues.iter().collect();
        order.sort_by_key(|c| (c.number, c.dir == Dir::Down));
        if order.len() != texts.len() {
            return Err(ParseError::BadGrid);
        }
        let mut across = Vec::new();
        let mut down = Vec::new();
        for (clue, text) in order.into_iter().zip(texts) {
            match clue.dir {
                Dir::Across => across.push((clue.number, text)),
                Dir::Down => down.push((clue.number, text)),
            }
        }
        let mut p = Puzzle::new(width, height, solution, circled, &across, &down)?;
        p.title = title;
        p.author = author;
        p.copyright = copyright;
        p.notes = notes;
        Ok(p)
    }

    /// Read an ipuz crossword (ipuz.org): JSON with the dimensions, a puzzle grid ("#" for a
    /// block, a cell object whose style may circle it), a solution grid, and clue lists keyed
    /// by direction. The grid is numbered here rather than trusted, and each clue lands on the
    /// word its number starts.
    pub fn from_ipuz(text: &str) -> Result<Puzzle, ParseError> {
        use serde_json::Value;
        let v: Value = serde_json::from_str(text).map_err(|_| ParseError::NotAPuzzle)?;
        let dims = v.get("dimensions").ok_or(ParseError::NotAPuzzle)?;
        let width = dims.get("width").and_then(Value::as_u64).unwrap_or(0) as usize;
        let height = dims.get("height").and_then(Value::as_u64).unwrap_or(0) as usize;
        let rows = |key: &str| -> Result<Vec<Value>, ParseError> {
            let grid = v
                .get(key)
                .and_then(Value::as_array)
                .ok_or(ParseError::BadGrid)?;
            let flat: Vec<Value> = grid
                .iter()
                .flat_map(|row| row.as_array().cloned().unwrap_or_default())
                .collect();
            if flat.len() == width * height {
                Ok(flat)
            } else {
                Err(ParseError::BadGrid)
            }
        };
        let block = v.get("block").and_then(Value::as_str).unwrap_or("#");
        let puzzle = rows("puzzle")?;
        let answers = rows("solution")?;
        let is_block = |cell: &Value| match cell {
            Value::String(s) => s == block,
            Value::Null => true,
            Value::Object(o) => o.get("cell").and_then(Value::as_str) == Some(block),
            _ => false,
        };
        let mut solution = Vec::with_capacity(width * height);
        let mut circled = Vec::with_capacity(width * height);
        for (cell, answer) in puzzle.iter().zip(&answers) {
            let value = match answer {
                Value::String(s) => s.clone(),
                Value::Object(o) => o
                    .get("value")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                _ => String::new(),
            };
            if is_block(cell) || value == block || value.is_empty() {
                solution.push(String::new());
                circled.push(false);
            } else {
                solution.push(value.to_uppercase());
                circled.push(
                    cell.get("style")
                        .and_then(|s| s.get("shapebg"))
                        .and_then(Value::as_str)
                        == Some("circle"),
                );
            }
        }
        let clue_list = |dir: &str| -> Vec<(u16, String)> {
            let clues = v.get("clues").and_then(Value::as_object);
            let list = clues.and_then(|m| {
                m.iter()
                    .find(|(k, _)| k.split(':').next() == Some(dir))
                    .and_then(|(_, list)| list.as_array())
            });
            list.map(|l| {
                l.iter()
                    .filter_map(|c| match c {
                        Value::Array(pair) => Some((
                            number_of(pair.first()?)?,
                            pair.get(1)?.as_str()?.to_string(),
                        )),
                        Value::Object(o) => Some((
                            number_of(o.get("number")?)?,
                            o.get("clue")?.as_str()?.to_string(),
                        )),
                        _ => None,
                    })
                    .map(|(n, t)| (n, plain_text(&t)))
                    .collect()
            })
            .unwrap_or_default()
        };
        let across = clue_list("Across");
        let down = clue_list("Down");
        let mut p = Puzzle::new(width, height, solution, circled, &across, &down)?;
        let field = |k: &str| {
            v.get(k)
                .and_then(Value::as_str)
                .map(plain_text)
                .unwrap_or_default()
        };
        p.title = field("title");
        p.author = field("author");
        p.copyright = field("copyright");
        p.notes = field("notes");
        Ok(p)
    }
}

fn number_of(v: &serde_json::Value) -> Option<u16> {
    match v {
        serde_json::Value::Number(n) => n.as_u64().map(|n| n as u16),
        serde_json::Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// ipuz text may carry a little HTML (italics for titles, entities); a clue reads as plain text.
pub fn plain_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
}

/// What a check or reveal left on a square.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Mark {
    #[default]
    None,
    /// Checked and wrong: drawn with a slash until the square is changed.
    Wrong,
    /// Checked and right: locked, so it cannot be typed over by accident.
    Right,
    /// Filled in by a reveal: locked, and drawn with a corner flag.
    Revealed,
}

/// How much a check, reveal or clear covers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope {
    Square,
    Word,
    Puzzle,
}

/// The durable part of a solve (gamekit save/restore), keyed by the puzzle's id.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    pub entries: Vec<String>,
    pub pencil: Vec<bool>,
    pub marks: Vec<Mark>,
    pub cursor: usize,
    pub dir: Dir,
    pub elapsed: f64,
    pub solved: bool,
    /// A reveal or a check was used, which the solved card mentions.
    pub assisted: bool,
}

impl Progress {
    pub fn fresh(p: &Puzzle) -> Progress {
        let n = p.width * p.height;
        Progress {
            entries: vec![String::new(); n],
            pencil: vec![false; n],
            marks: vec![Mark::None; n],
            cursor: (0..n).find(|&i| !p.is_block(i)).unwrap_or(0),
            ..Progress::default()
        }
    }

    /// How much of the grid is filled, 0 to 1, for the library's progress rings.
    pub fn filled(&self, p: &Puzzle) -> f64 {
        let open = p.open_count();
        if open == 0 {
            return 0.0;
        }
        let done = (0..self.entries.len())
            .filter(|&i| !p.is_block(i) && !self.entries[i].is_empty())
            .count();
        done as f64 / open as f64
    }
}

/// A solve in progress: the puzzle, its progress, and the input state around them.
pub struct Solve {
    pub puzzle: Puzzle,
    pub progress: Progress,
    /// Letters typed now go in as pencil marks: lighter, for a guess.
    pub pencil_mode: bool,
    /// A rebus square is being typed: letters add to the square rather than replace it.
    pub rebus: bool,
    pub paused: bool,
}

impl Solve {
    pub fn new(puzzle: Puzzle, saved: Option<Progress>) -> Solve {
        let fresh = Progress::fresh(&puzzle);
        let progress = match saved {
            Some(s) if s.entries.len() == fresh.entries.len() => s,
            _ => fresh,
        };
        let mut s = Solve {
            puzzle,
            progress,
            pencil_mode: false,
            rebus: false,
            paused: false,
        };
        s.settle_dir();
        s
    }

    pub fn cursor(&self) -> usize {
        self.progress.cursor
    }

    pub fn dir(&self) -> Dir {
        self.progress.dir
    }

    /// The clue under the cursor in the current direction.
    pub fn current_clue(&self) -> Option<usize> {
        self.puzzle.clue_at(self.progress.cursor, self.dir())
    }

    /// The crossing clue: the other direction's word through the cursor.
    pub fn crossing_clue(&self) -> Option<usize> {
        self.puzzle
            .clue_at(self.progress.cursor, self.progress.dir.flip())
    }

    pub fn tick(&mut self, dt: f64) {
        if !self.paused && !self.progress.solved {
            self.progress.elapsed += dt;
        }
    }

    /// A square standing alone in one direction has no word that way: turn to the one it has.
    fn settle_dir(&mut self) {
        if self.current_clue().is_none() {
            self.progress.dir = self.progress.dir.flip();
        }
    }

    fn locked(&self, i: usize) -> bool {
        self.progress.solved || matches!(self.progress.marks[i], Mark::Right | Mark::Revealed)
    }

    /// Tap or click a square: a new square takes the cursor, the cursor's own square turns the
    /// direction around.
    pub fn select(&mut self, i: usize) {
        if i >= self.puzzle.solution.len() || self.puzzle.is_block(i) {
            return;
        }
        self.rebus = false;
        if i == self.progress.cursor {
            self.toggle_dir();
        } else {
            self.progress.cursor = i;
            self.settle_dir();
        }
    }

    /// Turn to the other direction, where the cursor's square has a word that way.
    pub fn toggle_dir(&mut self) {
        if self.crossing_clue().is_some() {
            self.progress.dir = self.progress.dir.flip();
        }
    }

    /// Jump to a clue from the list: its first empty square, else its first square.
    pub fn select_clue(&mut self, k: usize) {
        let Some(clue) = self.puzzle.clues.get(k) else {
            return;
        };
        self.rebus = false;
        self.progress.dir = clue.dir;
        self.progress.cursor = clue
            .cells
            .iter()
            .copied()
            .find(|&i| self.progress.entries[i].is_empty())
            .unwrap_or(clue.cells[0]);
    }

    /// The next (or previous) clue in list order, wrapping from the last Down to the first
    /// Across, skipping words already full when any are left to fill.
    pub fn next_clue(&mut self, forward: bool) {
        let n = self.puzzle.clues.len();
        if n == 0 {
            return;
        }
        let here = self.current_clue().unwrap_or(0);
        let step = |k: usize| {
            if forward {
                (k + 1) % n
            } else {
                (k + n - 1) % n
            }
        };
        let full = |k: usize| {
            self.puzzle.clues[k]
                .cells
                .iter()
                .all(|&i| !self.progress.entries[i].is_empty())
        };
        let mut k = step(here);
        while k != here && full(k) {
            k = step(k);
        }
        if k == here {
            k = step(here);
        }
        self.select_clue(k);
    }

    /// Type a letter (or, in rebus mode, add one to the square).
    pub fn type_letter(&mut self, ch: char) {
        if !ch.is_alphanumeric() || self.paused {
            return;
        }
        let i = self.progress.cursor;
        if self.locked(i) {
            self.advance();
            return;
        }
        let ch = ch.to_uppercase().to_string();
        if self.rebus {
            self.progress.entries[i].push_str(&ch);
        } else {
            self.progress.entries[i] = ch;
        }
        self.progress.pencil[i] = self.pencil_mode;
        if self.progress.marks[i] == Mark::Wrong {
            self.progress.marks[i] = Mark::None;
        }
        if !self.rebus {
            self.advance();
        }
        self.check_solved();
    }

    /// End a rebus square and move on.
    pub fn finish_rebus(&mut self) {
        if self.rebus {
            self.rebus = false;
            self.advance();
            self.check_solved();
        }
    }

    /// After a letter: the next empty square in the word (wrapping to its start), else the next
    /// square, else the next word that still has room.
    fn advance(&mut self) {
        let Some(k) = self.current_clue() else {
            return;
        };
        let cells = &self.puzzle.clues[k].cells;
        let pos = cells
            .iter()
            .position(|&i| i == self.progress.cursor)
            .unwrap_or(0);
        let empty = |i: usize| self.progress.entries[i].is_empty();
        if let Some(&i) = cells[pos + 1..].iter().find(|&&i| empty(i)) {
            self.progress.cursor = i;
        } else if let Some(&i) = cells[..pos].iter().find(|&&i| empty(i)) {
            self.progress.cursor = i;
        } else if pos + 1 < cells.len() {
            self.progress.cursor = cells[pos + 1];
        } else {
            self.next_clue(true);
        }
    }

    /// Backspace: clear the square under the cursor, or, when it is already empty, step back a
    /// square in the word (into the previous word from a word's first square) and clear that.
    pub fn backspace(&mut self) {
        if self.paused {
            return;
        }
        let i = self.progress.cursor;
        if self.rebus && !self.progress.entries[i].is_empty() {
            self.progress.entries[i].pop();
            return;
        }
        self.rebus = false;
        if !self.progress.entries[i].is_empty() && !self.locked(i) {
            self.clear_square(i);
            return;
        }
        let Some(k) = self.current_clue() else {
            return;
        };
        let cells = self.puzzle.clues[k].cells.clone();
        let pos = cells.iter().position(|&c| c == i).unwrap_or(0);
        let back = if pos > 0 {
            cells[pos - 1]
        } else {
            let n = self.puzzle.clues.len();
            let prev = &self.puzzle.clues[(k + n - 1) % n];
            self.progress.dir = prev.dir;
            prev.cells[prev.cells.len() - 1]
        };
        self.progress.cursor = back;
        if !self.locked(back) {
            self.clear_square(back);
        }
    }

    fn clear_square(&mut self, i: usize) {
        self.progress.entries[i].clear();
        self.progress.pencil[i] = false;
        if self.progress.marks[i] == Mark::Wrong {
            self.progress.marks[i] = Mark::None;
        }
    }

    /// An arrow key. Across the current direction it only turns (the way most crossword apps
    /// behave, so a stray arrow never loses your place); along it, it moves to the next open
    /// square, jumping blocks.
    pub fn arrow(&mut self, dr: isize, dc: isize) {
        self.rebus = false;
        let dir = if dr == 0 { Dir::Across } else { Dir::Down };
        if dir != self.progress.dir && self.puzzle.clue_at(self.progress.cursor, dir).is_some() {
            self.progress.dir = dir;
            return;
        }
        let (w, h) = (self.puzzle.width as isize, self.puzzle.height as isize);
        let mut r = (self.progress.cursor / self.puzzle.width) as isize;
        let mut c = (self.progress.cursor % self.puzzle.width) as isize;
        loop {
            r += dr;
            c += dc;
            if r < 0 || c < 0 || r >= h || c >= w {
                return;
            }
            let i = (r * w + c) as usize;
            if !self.puzzle.is_block(i) {
                self.progress.cursor = i;
                self.settle_dir();
                return;
            }
        }
    }

    fn scope_cells(&self, scope: Scope) -> Vec<usize> {
        match scope {
            Scope::Square => vec![self.progress.cursor],
            Scope::Word => self
                .current_clue()
                .map(|k| self.puzzle.clues[k].cells.clone())
                .unwrap_or_default(),
            Scope::Puzzle => (0..self.puzzle.solution.len())
                .filter(|&i| !self.puzzle.is_block(i))
                .collect(),
        }
    }

    fn right(&self, i: usize) -> bool {
        let want = &self.puzzle.solution[i];
        let got = &self.progress.entries[i];
        // A rebus answer also takes its first letter, the way Across Lite scores one.
        got == want
            || (want.chars().count() > 1
                && got.chars().count() == 1
                && want.starts_with(got.as_str()))
    }

    /// Check the squares in `scope`: wrong letters get a slash, right ones lock. Empty squares
    /// are left alone. Returns how many were wrong.
    pub fn check(&mut self, scope: Scope) -> usize {
        let mut wrong = 0;
        for i in self.scope_cells(scope) {
            if self.progress.entries[i].is_empty() || self.progress.marks[i] == Mark::Revealed {
                continue;
            }
            if self.right(i) {
                self.progress.marks[i] = Mark::Right;
                self.progress.pencil[i] = false;
            } else {
                self.progress.marks[i] = Mark::Wrong;
                wrong += 1;
            }
        }
        self.progress.assisted = true;
        wrong
    }

    /// Fill the squares in `scope` with their answers, flagged as revealed (a right letter
    /// already there stays the solver's).
    pub fn reveal(&mut self, scope: Scope) {
        for i in self.scope_cells(scope) {
            if !self.right(i) {
                self.progress.entries[i] = self.puzzle.solution[i].clone();
                self.progress.marks[i] = Mark::Revealed;
                self.progress.pencil[i] = false;
            }
        }
        self.progress.assisted = true;
        self.check_solved();
    }

    /// Empty the squares in `scope` that are not locked.
    pub fn clear(&mut self, scope: Scope) {
        for i in self.scope_cells(scope) {
            if !self.locked(i) {
                self.clear_square(i);
            }
        }
    }

    /// Start over: every square empty, every mark gone, the clock at zero.
    pub fn restart(&mut self) {
        self.progress = Progress::fresh(&self.puzzle);
        self.pencil_mode = false;
        self.rebus = false;
        self.settle_dir();
    }

    fn check_solved(&mut self) {
        if self.progress.solved {
            return;
        }
        let all = (0..self.puzzle.solution.len()).all(|i| self.puzzle.is_block(i) || self.right(i));
        if all {
            self.progress.solved = true;
            self.rebus = false;
        }
    }

    /// Every open square has something in it, but the grid is not right: the cue to check.
    pub fn full_but_wrong(&self) -> bool {
        !self.progress.solved
            && (0..self.puzzle.solution.len())
                .all(|i| self.puzzle.is_block(i) || !self.progress.entries[i].is_empty())
    }
}

/// mm:ss (or h:mm:ss past an hour) for the clock and the library.
pub fn fmt_time(secs: u64) -> String {
    if secs >= 3600 {
        format!("{}:{:02}:{:02}", secs / 3600, (secs / 60) % 60, secs % 60)
    } else {
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 3×3 with a block in the middle:
    ///   C A T
    ///   A # O
    ///   B E D
    fn mini() -> Puzzle {
        let grid = ["C", "A", "T", "A", "", "O", "B", "E", "D"];
        let solution = grid.iter().map(|s| s.to_string()).collect();
        Puzzle::new(
            3,
            3,
            solution,
            Vec::new(),
            &[(1, "Feline".into()), (4, "Sleeping place".into())],
            &[(1, "Taxi".into()), (2, "Fastened".into())],
        )
        .unwrap()
    }

    #[test]
    fn numbers_the_grid_the_standard_way() {
        let p = mini();
        assert_eq!(p.numbers, vec![1, 0, 2, 0, 0, 0, 3, 0, 0]);
        let names: Vec<_> = p.clues.iter().map(|c| (c.number, c.dir)).collect();
        assert_eq!(
            names,
            vec![
                (1, Dir::Across),
                (3, Dir::Across),
                (1, Dir::Down),
                (2, Dir::Down)
            ]
        );
        assert_eq!(p.clues[1].cells, vec![6, 7, 8]);
        assert_eq!(p.clues[3].cells, vec![2, 5, 8]);
    }

    #[test]
    fn clue_text_lands_on_the_number_it_names() {
        // Our numbering gives the bottom row 3, so the "4 Across" text finds no word.
        let p = mini();
        assert_eq!(p.clues[0].text, "Feline");
        assert_eq!(p.clues[1].text, "");
        assert_eq!(p.clues[2].text, "Taxi");
    }

    #[test]
    fn typing_fills_the_word_then_moves_on() {
        let mut s = Solve::new(mini(), None);
        assert_eq!((s.cursor(), s.dir()), (0, Dir::Across));
        for ch in "CAT".chars() {
            s.type_letter(ch);
        }
        assert_eq!(s.progress.entries[..3], ["C", "A", "T"].map(String::from));
        // The word is full, so the cursor went on to the next word with room: 3 Across.
        assert_eq!((s.cursor(), s.dir()), (6, Dir::Across));
    }

    #[test]
    fn typing_skips_filled_squares_in_the_word() {
        let mut s = Solve::new(mini(), None);
        s.select(1);
        s.type_letter('a');
        assert_eq!(s.cursor(), 2);
        s.select(0);
        s.type_letter('c');
        // The A is already there, so the cursor skips it to the empty T.
        assert_eq!(s.cursor(), 2);
    }

    #[test]
    fn tapping_the_cursor_turns_it_around() {
        let mut s = Solve::new(mini(), None);
        s.select(0);
        assert_eq!(s.dir(), Dir::Down);
        // The middle of the top row has no Down word, so it stays Across.
        s.select(1);
        assert_eq!(s.dir(), Dir::Across);
        s.select(1);
        assert_eq!(s.dir(), Dir::Across);
    }

    #[test]
    fn backspace_clears_then_steps_back() {
        let mut s = Solve::new(mini(), None);
        s.type_letter('C');
        s.type_letter('A');
        assert_eq!(s.cursor(), 2);
        s.backspace();
        assert_eq!((s.cursor(), s.progress.entries[1].as_str()), (1, ""));
        s.backspace();
        assert_eq!((s.cursor(), s.progress.entries[0].as_str()), (0, ""));
    }

    #[test]
    fn an_arrow_across_the_direction_only_turns() {
        let mut s = Solve::new(mini(), None);
        s.arrow(1, 0);
        assert_eq!((s.cursor(), s.dir()), (0, Dir::Down));
        s.arrow(1, 0);
        assert_eq!(s.cursor(), 3);
        // Down from row 1, column 0 continues to row 2.
        s.arrow(1, 0);
        assert_eq!(s.cursor(), 6);
        // Right along row 2 after turning.
        s.arrow(0, 1);
        assert_eq!(s.dir(), Dir::Across);
        s.arrow(0, 1);
        assert_eq!(s.cursor(), 7);
    }

    #[test]
    fn check_marks_and_locks() {
        let mut s = Solve::new(mini(), None);
        for ch in "CXT".chars() {
            s.type_letter(ch);
        }
        s.select(0);
        assert_eq!(s.dir(), Dir::Across);
        assert_eq!(s.check(Scope::Word), 1);
        assert_eq!(
            s.progress.marks[..3],
            [Mark::Right, Mark::Wrong, Mark::Right]
        );
        // A locked square keeps its letter; a wrong one takes a new letter and loses its slash.
        s.select(0);
        s.type_letter('Z');
        assert_eq!(s.progress.entries[0], "C");
        s.select(1);
        s.type_letter('A');
        assert_eq!(s.progress.marks[1], Mark::None);
    }

    #[test]
    fn reveal_and_finish() {
        let mut s = Solve::new(mini(), None);
        s.reveal(Scope::Puzzle);
        assert!(s.progress.solved);
        assert!(s.progress.assisted);
        assert_eq!(s.progress.marks[0], Mark::Revealed);
    }

    #[test]
    fn solving_by_typing_is_detected() {
        let mut s = Solve::new(mini(), None);
        s.select_clue(0);
        for ch in "CAT".chars() {
            s.type_letter(ch);
        }
        s.select_clue(1);
        for ch in "BED".chars() {
            s.type_letter(ch);
        }
        s.select_clue(2);
        s.type_letter('A');
        assert!(!s.progress.solved);
        s.select_clue(3);
        s.type_letter('O');
        assert!(s.progress.solved);
        assert!(!s.progress.assisted);
    }

    #[test]
    fn rebus_squares_take_several_letters() {
        let solution = ["HEART", "S", "", "X"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let p = Puzzle::new(2, 2, solution, Vec::new(), &[], &[]).unwrap();
        let mut s = Solve::new(p, None);
        s.rebus = true;
        for ch in "HEART".chars() {
            s.type_letter(ch);
        }
        assert_eq!(s.progress.entries[0], "HEART");
        s.finish_rebus();
        assert_eq!(s.cursor(), 1);
    }

    /// A hand-built Across Lite file for the mini, with a circle on the middle of the top row.
    fn mini_puz() -> Vec<u8> {
        let mut b = vec![0u8; 0x34];
        b[0x02..0x0E].copy_from_slice(b"ACROSS&DOWN\0");
        b[0x18..0x1C].copy_from_slice(b"1.3\0");
        b[0x2C] = 3;
        b[0x2D] = 3;
        b[0x2E] = 4;
        b.extend_from_slice(b"CATA.OBED");
        b.extend_from_slice(b"----.----");
        for s in [
            "Mini",
            "A. Setter",
            "© 2026",
            "Feline",
            "Taxi",
            "Fastened",
            "Sleeping place",
            "",
        ] {
            b.extend_from_slice(s.as_bytes());
            b.push(0);
        }
        b.extend_from_slice(b"GEXT");
        b.extend_from_slice(&9u16.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(&[0, 0x80, 0, 0, 0, 0, 0, 0, 0]);
        b.push(0);
        b
    }

    #[test]
    fn reads_an_across_lite_file() {
        let p = Puzzle::from_puz(&mini_puz()).unwrap();
        assert_eq!((p.width, p.height), (3, 3));
        assert_eq!(p.title, "Mini");
        assert_eq!(p.author, "A. Setter");
        assert!(p.circled[1]);
        let texts: Vec<_> = p
            .clues
            .iter()
            .map(|c| (c.number, c.text.as_str()))
            .collect();
        assert_eq!(
            texts,
            vec![
                (1, "Feline"),
                (3, "Sleeping place"),
                (1, "Taxi"),
                (2, "Fastened")
            ]
        );
    }

    #[test]
    fn refuses_a_scrambled_file() {
        let mut b = mini_puz();
        b[0x32] = 4;
        assert_eq!(Puzzle::from_puz(&b), Err(ParseError::Scrambled));
    }

    #[test]
    fn reads_an_ipuz_file() {
        let text = r##"{
          "version": "http://ipuz.org/v2",
          "kind": ["http://ipuz.org/crossword#1"],
          "title": "<i>Mini</i>", "author": "A. Setter",
          "dimensions": {"width": 3, "height": 3},
          "puzzle": [[1, {"cell": 0, "style": {"shapebg": "circle"}}, 2], [0, "#", 0], [3, 0, 0]],
          "solution": [["C","A","T"], ["A","#","O"], ["B","E","D"]],
          "clues": {"Across": [[1, "Feline"], {"number": 3, "clue": "Sleeping &amp; dreaming place"}],
                    "Down": [["1", "Taxi"], [2, "Fastened"]]}
        }"##;
        let p = Puzzle::from_ipuz(text).unwrap();
        assert_eq!(p.title, "Mini");
        assert!(p.circled[1]);
        assert_eq!(p.clues[1].text, "Sleeping & dreaming place");
        assert_eq!(p.clues[2].text, "Taxi");
    }

    #[test]
    fn saved_progress_comes_back() {
        let mut s = Solve::new(mini(), None);
        s.type_letter('C');
        let saved = s.progress.clone();
        let back = Solve::new(mini(), Some(saved.clone()));
        assert_eq!(back.progress, saved);
        assert!((back.progress.filled(&back.puzzle) - 1.0 / 8.0).abs() < 1e-9);
    }
}
