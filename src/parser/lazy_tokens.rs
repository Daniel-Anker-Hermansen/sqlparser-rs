use core::cell::RefCell;
use std::{collections::VecDeque, fs::File, io::{BufReader, Bytes, Read}};

use super::{Dialect, State, TokenWithLocation, Tokenizer};

pub struct LazyTokens<'a> {
    tokenizer: Tokenizer<'a>,
    buf: RefCell<VecDeque<TokenWithLocation>>,
    buf_index: usize,
    state: RefCell<State<Utf8Iter>>,
}

impl<'a> LazyTokens<'a> {
    pub fn new(dialect: &'a dyn Dialect, file: File) -> Self {
        Self {
            tokenizer: Tokenizer::new(dialect),
            buf: RefCell::new(VecDeque::new()),
            buf_index: 0,
            state: RefCell::new(State {
                peekable: LookAheadIterator { iter: Utf8Iter { bytes: BufReader::new(file).bytes() }, buf: VecDeque::new() },
                line: 1,
                col: 1,
            }),
        }
    }

    pub fn get(&self, index: usize) -> Option<TokenWithLocation> {
        let true_index = index - self.buf_index;
        while self.buf.borrow().len() <= true_index {
            let token = self
                .tokenizer
                .next_token(&mut self.state.borrow_mut())
                .expect("no tokenizer error");
            match token {
                Some(token) => self.buf.borrow_mut().push_back(TokenWithLocation {
                    token,
                    location: self.state.borrow().location(),
                }),
                None => return None,
            }
        }
        self.buf.borrow().get(true_index).cloned()
    }

    pub fn clear(&mut self, index: usize) {
        let difference = index - self.buf_index;
        self.buf_index = index;
        for _ in 0..difference {
            self.buf.borrow_mut().pop_front();
        }
    }
}

struct Utf8Iter {
	bytes: Bytes<BufReader<File>>,
}

impl Iterator for Utf8Iter {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
	let first = self.bytes.next()?.ok()?;
	let leading = first.leading_ones();
	let mut c = ((first << leading) >> leading) as u32;
	for _ in 1..leading {
	    let b = self.bytes.next()?.ok()?;
	    c = (c << 6) | (((b << 1) >> 1) as u32);
	}
	char::from_u32(c)
    }
}

pub struct LookAheadIterator<I> where I: Iterator {
	iter: I,
	buf: VecDeque<I::Item>,
}


impl<I> Iterator for LookAheadIterator<I> where I: Iterator {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.buf.is_empty() {
	    self.buf.pop_front()
	}
	else {
	    self.iter.next()
	}
    }
}

impl<I> LookAheadIterator<I> where I: Iterator {
    pub fn new(iter: I) -> Self {
	LookAheadIterator { iter, buf: VecDeque::new() }
    }

    pub fn peek(&mut self) -> Option<&<Self as Iterator>::Item> {
	if self.buf.is_empty() {
	    self.fill_buf();
	}
	self.buf.get(0)
    }

    fn fill_buf(&mut self) -> Option<()> {
	self.buf.push_back(self.iter.next()?);
	Some(())
    }

    pub fn lookahead(&mut self) -> LookAheadHandle<I> {
	LookAheadHandle { inner: self, index: 0 }
    }
}

pub struct LookAheadHandle<'a, I> where I: Iterator {
	inner: &'a mut LookAheadIterator<I>,
	index: usize,
}

impl<'a, I> Iterator for LookAheadHandle<'a, I> where I: Iterator, <I as Iterator>::Item: Clone {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
	while self.inner.buf.len() < self.index {
	    self.inner.fill_buf()?;
	}
	self.index += 1;
	self.inner.buf.get(self.index - 1).cloned()
    }
}
