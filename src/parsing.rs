use std::io::{self, BufRead};
use std::str::FromStr;

use nom::character::streaming::{space0, space1, digit1, hex_digit1, not_line_ending};
use nom::number::streaming::float;

use crate::{Call, Allocation, Location};

pub fn massif_tree<R: BufRead>(reader: R) -> Iter<R> {
    Iter {
        reader,
        lineno: 0,
        line: String::new(),
        calls: Vec::new(),
    }
}

pub struct Iter<R> {
    reader: R,
    lineno: usize,
    line: String,
    calls: Vec<(Call, usize)>,
}

macro_rules! try_iter {
    ($x:expr) => {
        match $x {
            Ok(x)  => { x }
            Err(e) => { return Some(Err(e)); }
        }
    }
}

impl<R: BufRead> Iterator for Iter<R> {
    type Item = io::Result<(Call, Option<Call>, Allocation)>;

    fn next(&mut self) -> Option<Self::Item> {
        if try_iter!(self.reader.read_line(&mut self.line)) == 0 {
            return None;
        }

        self.lineno += 1;
        let parsed_line = try_iter!(massif_line(&self.line)
            .map(|(_, parsed)| parsed)
            .map_err(|e| {
                io::Error::new(io::ErrorKind::Other,
                               format!("Failed to parse line {}: {:?}", self.lineno, e))
            }));

        let callee;
        loop {
            if let Some((_, ref mut nb_callers)) = self.calls.last_mut() {
                if *nb_callers == 0 {
                    self.calls.pop();
                    continue;
                }

                *nb_callers -= 1;
            }

            callee = self.calls.last();
            break;
        }

        let callee = callee.map(|(c, _)| c.clone());

        let (caller, location) = split_symbol(parsed_line.symbol);

        let allocation = Allocation::new(parsed_line.bytes, location);

        self.calls.push((caller.clone(), parsed_line.nb_callers));
        self.line.clear();

        Some(Ok((caller, callee, allocation)))
    }
}

fn split_symbol(symbol: Symbol) -> (Call, Location) {
    use Symbol::*;

    let call = match symbol {
        Internal((address, _)) => Call::Inner(address.to_string()),
        External(_)            => Call::Leaf,
        Ignored(_)             => Call::Root,
    };

    use Location::*;
    let location = match symbol {
        External(description) | Internal((_, description)) => Described(description.to_string()),
        Ignored((count, threshold))                        => Omitted((count, threshold)),
    };

    (call, location)
}

#[derive(Debug)]
pub struct Line<'a> {
    nb_callers: usize,
    bytes: usize,
    symbol: Symbol<'a>,
}

impl<'a> From<LineTuple<'a>> for Line<'a> {
    fn from(tuple: LineTuple<'a>) -> Self {
        let (nb_callers, bytes, symbol) = tuple;
        Line { nb_callers, bytes, symbol }
    }
}

type LineTuple<'a> = (usize, usize, Symbol<'a>);

#[derive(Debug, PartialEq)]
pub enum Symbol<'a> {
    External(&'a str),
    Internal((&'a str, &'a str)),
    Ignored((usize, f32)),
}

named!(massif_line<&str, Line>,
       map!(massif_line_tuple, Line::from));

named!(massif_line_tuple<&str, LineTuple>,
       terminated!(tuple!(nb_callers, bytes, symbol),
                   do_parse!(opt!(char!('\r')) >> char!('\n') >> ())));

named!(nb_callers<&str, usize>,
       map_res!(preceded!(space0, terminated!(preceded!(char!('n'), digit1), char!(':'))),
                usize::from_str));

named!(bytes<&str, usize>,
       map_res!(preceded!(space0, digit1),
                usize::from_str));

named!(symbol<&str, Symbol>,
       preceded!(space0,
                 alt!(map!(ignored_places, Symbol::Ignored)
                      |
                      map!(internal_address, Symbol::Internal)
                      |
                      map!(external_call, Symbol::External))));

named!(ignored_places<&str, (usize, f32)>,
       do_parse!(
           tag!("in ")                                  >>
           nb_places: map_res!(digit1, usize::from_str) >>
           tag!(" place") >> opt!(char!('s'))           >>
           tag!(", ") >> opt!(tag!("all "))             >>
           tag!("below massif's threshold (")           >>
           threshold: float                             >>
           tag!("%)")                                   >>
           (nb_places, threshold)));

named!(internal_address<&str, (&str, &str)>,
       do_parse!(
           address: hex_address >> char!(':') >> space1 >>
           description: description >>
           (address, description)));

named!(hex_address<&str, &str>,
       preceded!(tag!("0x"), hex_digit1));

named!(external_call<&str, &str>, call!(description));

named!(description<&str, &str>, call!(not_line_ending));

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn it_parses_the_number_of_callers() {
        assert_eq!(nb_callers(" n123: "), Ok((" ", 123)));
    }

    #[test]
    fn it_parses_the_number_of_bytes() {
        assert_eq!(bytes(" 456 "), Ok((" ", 456)));
    }

    #[test]
    fn it_parses_ignored_places() {
        assert_eq!(ignored_places("in 1 place, below massif's threshold (0.01%)"), Ok(("", (1, 0.01))))
    }

    #[test]
    fn it_parses_symbols() {
        use Symbol::*;
        assert_eq!(symbol("(heap allocation functions) malloc/new/new[], --alloc-fns, etc.\n"), Ok(("\n", External("(heap allocation functions) malloc/new/new[], --alloc-fns, etc."))));
        assert_eq!(symbol("0x4E23FC67: std::string::_Rep::_S_create(unsigned long, unsigned long, std::allocator<char> const&) (in libstdc++.so)\n"), Ok(("\n", Internal(("4E23FC67", "std::string::_Rep::_S_create(unsigned long, unsigned long, std::allocator<char> const&) (in libstdc++.so)")))));
        assert_eq!(symbol("in 1 place, below massif's threshold (0.01%)"), Ok(("", Ignored((1, 0.01)))));
        assert_eq!(symbol("in 5 places, below massif's threshold (0.01%)"), Ok(("", Ignored((5, 0.01)))));
        assert_eq!(symbol("in 9570 places, all below massif's threshold (0.01%)\n"),
                   Ok(("\n", Ignored((9570, 0.01)))));
    }

    #[test]
    fn it_parses_lines() {
        use Symbol::*;
        assert_eq!(massif_line_tuple("n184: 94985897 (heap allocation functions) malloc/new/new[], --alloc-fns, etc.\n").map(|(_, o)| o),
                   Ok((184, 94985897, External("(heap allocation functions) malloc/new/new[], --alloc-fns, etc."))));
        assert_eq!(massif_line_tuple("n4: 13847645 0x4E23FC67: std::string::_Rep::_S_create(unsigned long, unsigned long, std::allocator<char> const&) (in libstdc++.so)\n").map(|(_, o)| o),
                   Ok((4, 13847645, Internal(("4E23FC67", "std::string::_Rep::_S_create(unsigned long, unsigned long, std::allocator<char> const&) (in libstdc++.so)")))));
        assert_eq!(massif_line_tuple("n0: 109 in 1 place, below massif's threshold (0.01%)\n").map(|(_, o)| o),
                   Ok((0, 109, Ignored((1, 0.01)))));
        assert_eq!(massif_line_tuple("n0: 1355955 in 9570 places, all below massif's threshold (0.01%)\n").map(|(_, o)| o),
                   Ok((0, 1355955, Ignored((9570, 0.01)))));
    }

    #[test]
    fn it_works() {
        // note: this smoke test may not be 100% accurate
        let tree = r#"
        n2: 11592561 0x15266383: char* std::string::_S_construct<char const*>(char const*, char const*, std::allocator<char> const&, std::forward_iterator_tag) (in liblog4cxx.so)
         n0: 11592452 0x4E241956: std::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string(char const*, std::allocator<char> const&) (in libstdc++.so)
         n0: 109 in 1 place, below massif's threshold (0.01%)
        n0: 42 in 5 places, below massif's threshold (0.01%)
         "#.trim_start().trim_end_matches(' ');

        use Location::*;
        let root1 = Allocation::new(11592561, Described("char* std::string::_S_construct<char const*>(char const*, char const*, std::allocator<char> const&, std::forward_iterator_tag) (in liblog4cxx.so)".to_string()));
        let child1 = Allocation::new(11592452, Described("std::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string(char const*, std::allocator<char> const&) (in libstdc++.so)".to_string()));
        let child2 = Allocation::new(109, Omitted((1, 0.01)));
        let root2 = Allocation::new(42, Omitted((5, 0.01)));

        use Call::*;
        let reader = BufReader::new(tree.as_bytes());
        let mut calls = massif_tree(reader).map(|result| result.map_err(|e| format!("{:?}", e)));
        assert_eq!(calls.next(), Some(Ok((Inner("15266383".to_string()), None, root1))));
        assert_eq!(calls.next(), Some(Ok((Inner("4E241956".to_string()), Some(Inner("15266383".to_string())), child1))));
        assert_eq!(calls.next(), Some(Ok((Root, Some(Inner("15266383".to_string())), child2))));
        assert_eq!(calls.next(), Some(Ok((Root, None, root2))));
        assert_eq!(calls.next(), None);
    }
}
