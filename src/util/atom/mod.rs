/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::mem::replace;

mod data;

// Careful which things we derive, because we need to maintain equivalent
// behavior between an interned and a non-interned string.
/// Interned string.
#[deriving(Clone, Show)]
pub enum Atom {
    Static(&'static str),
    // dynamic interning goes here
    Owned(StrBuf),
}

impl Atom {
    pub fn from_str(s: &str) -> Atom {
        match data::atoms.find_key(&s) {
            Some(k) => Static(k),
            None => Owned(s.to_strbuf()),
        }
    }

    pub fn from_buf(s: StrBuf) -> Atom {
        match data::atoms.find_key(&s.as_slice()) {
            Some(k) => Static(k),
            None => Owned(s),
        }
    }

    /// Like `Atom::from_buf(replace(s, StrBuf::new()))` but avoids
    /// allocating a new `StrBuf` when the string is interned --
    /// just truncates the old one.
    pub fn take_from_buf(s: &mut StrBuf) -> Atom {
        match data::atoms.find_key(&s.as_slice()) {
            Some(k) => {
                s.truncate(0);
                Static(k)
            }
            None => {
                Owned(replace(s, StrBuf::new()))
            }
        }
    }

    #[inline(always)]
    fn fast_partial_eq(&self, other: &Atom) -> Option<bool> {
        match (self, other) {
            (&Static(x), &Static(y)) => Some(x.as_ptr() == y.as_ptr()),
            _ => None,
        }
    }
}

impl Str for Atom {
    fn as_slice<'t>(&'t self) -> &'t str {
        match *self {
            Static(s) => s,
            Owned(ref s) => s.as_slice(),
        }
    }

    fn into_owned(self) -> ~str {
        match self {
            Static(s) => s.into_owned(),
            Owned(s) => s.into_owned(),
        }
    }

    fn to_strbuf(&self) -> StrBuf {
        match *self {
            Static(s) => s.to_strbuf(),
            Owned(ref s) => s.clone(),
        }
    }

    fn into_strbuf(self) -> StrBuf {
        match self {
            Static(s) => s.into_strbuf(),
            Owned(s) => s,
        }
    }
}

impl Eq for Atom {
    fn eq(&self, other: &Atom) -> bool {
        match self.fast_partial_eq(other) {
            Some(b) => b,
            None => self.as_slice() == other.as_slice(),
        }
    }
}

impl TotalEq for Atom { }

impl Ord for Atom {
    fn lt(&self, other: &Atom) -> bool {
        match self.fast_partial_eq(other) {
            Some(true) => false,
            _ => self.as_slice() < other.as_slice(),
        }
    }
}

impl TotalOrd for Atom {
    fn cmp(&self, other: &Atom) -> Ordering {
        match self.fast_partial_eq(other) {
            Some(true) => Equal,
            _ => self.as_slice().cmp(&other.as_slice()),
        }
    }
}

#[test]
fn interned() {
    match Atom::from_str("body") {
        Static("body") => (),
        _ => fail!("wrong interning"),
    }
}

#[test]
fn not_interned() {
    match Atom::from_str("asdfghjk") {
        Owned(b) => assert_eq!(b.as_slice(), "asdfghjk"),
        _ => fail!("wrong interning"),
    }
}

#[test]
fn as_slice() {
    assert_eq!(Atom::from_str("").as_slice(), "");
    assert_eq!(Atom::from_str("body").as_slice(), "body");
    assert_eq!(Atom::from_str("asdfghjk").as_slice(), "asdfghjk");
}

#[test]
fn into_owned() {
    assert_eq!(Atom::from_str("").into_owned(), ~"");
    assert_eq!(Atom::from_str("body").into_owned(), ~"body");
    assert_eq!(Atom::from_str("asdfghjk").into_owned(), ~"asdfghjk");
}

#[test]
fn to_strbuf() {
    assert_eq!(Atom::from_str("").to_strbuf(), StrBuf::from_str(""));
    assert_eq!(Atom::from_str("body").to_strbuf(), StrBuf::from_str("body"));
    assert_eq!(Atom::from_str("asdfghjk").to_strbuf(), StrBuf::from_str("asdfghjk"));
}

#[test]
fn into_strbuf() {
    assert_eq!(Atom::from_str("").into_strbuf(), StrBuf::from_str(""));
    assert_eq!(Atom::from_str("body").into_strbuf(), StrBuf::from_str("body"));
    assert_eq!(Atom::from_str("asdfghjk").into_strbuf(), StrBuf::from_str("asdfghjk"));
}

#[test]
fn equality() {
    // Equality between interned and non-interned atoms
    assert!(Atom::from_str("body") == Owned(StrBuf::from_str("body")));
    assert!(Owned(StrBuf::from_str("body")) == Atom::from_str("body"));
    assert!(Atom::from_str("body") != Owned(StrBuf::from_str("asdfghjk")));
    assert!(Owned(StrBuf::from_str("asdfghjk")) != Atom::from_str("body"));
    assert!(Atom::from_str("asdfghjk") != Owned(StrBuf::from_str("body")));
    assert!(Owned(StrBuf::from_str("body")) != Atom::from_str("asdfghjk"));
}

#[test]
fn take_from_buf_interned() {
    let mut b = StrBuf::from_str("body");
    let a = Atom::take_from_buf(&mut b);
    assert_eq!(a, Atom::from_str("body"));
    assert_eq!(b, StrBuf::new());
}

#[test]
fn take_from_buf_not_interned() {
    let mut b = StrBuf::from_str("asdfghjk");
    let a = Atom::take_from_buf(&mut b);
    assert_eq!(a, Atom::from_str("asdfghjk"));
    assert_eq!(b, StrBuf::new());
}

#[test]
fn ord() {
    fn check(x: &str, y: &str) {
        assert_eq!(x < y, Atom::from_str(x) < Atom::from_str(y));
        assert_eq!(x.cmp(&y), Atom::from_str(x).cmp(&Atom::from_str(y)));
    }

    check("a", "body");
    check("asdf", "body");
    check("zasdf", "body");
    check("z", "body");

    check("a", "bbbbb");
    check("asdf", "bbbbb");
    check("zasdf", "bbbbb");
    check("z", "bbbbb");
}