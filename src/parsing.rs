use std::io::{self, BufRead};
use std::str::FromStr;

use nom::character::streaming::{space0, space1, digit1, hex_digit1, not_line_ending};
use nom::number::streaming::float;

pub(crate) fn massif_tree<R: BufRead>(reader: R) -> Iter<R> {
    Iter {
        reader,
        lineno: 0,
        line: String::new(),
        calls: Vec::new(),
    }
}

pub(crate) struct Iter<R> {
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

        // TODO group these calls under one match over the parsed line's symbol
        let call = Call::from_symbol(&parsed_line.symbol);
        let details = AllocationDetails::from_symbol(&parsed_line.symbol);

        let allocation = Allocation::new(parsed_line.bytes, details);

        self.calls.push((call.clone(), parsed_line.nb_callers));
        self.line.clear();

        Some(Ok((call, callee, allocation)))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Call {
    Inner(String),
    Leaf,
    Root,
}

impl Call {
    fn from_symbol(symbol: &Symbol) -> Self {
        match symbol {
            Internal((address, _)) => Call::Inner(address.to_string()),
            External(_)            => Call::Leaf,
            Ignored(_)             => Call::Root,
        }
    }

    #[cfg(debug_assertions)]
    pub fn is_leaf(&self) -> bool {
        if let Call::Leaf = self { true } else { false }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct Allocation {
    pub bytes: usize,
    pub details: AllocationDetails,
}

impl Allocation {
    fn new(bytes: usize, details: AllocationDetails) -> Self {
        Allocation { bytes, details }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum AllocationDetails {
    Described(String),
    Omitted((usize, f32)),
}
use AllocationDetails::*;

impl AllocationDetails {
    fn from_symbol(symbol: &Symbol) -> Self {
        match symbol {
            External(description) | Internal((_, description)) => Described(description.to_string()),
            Ignored((count, threshold))                        => Omitted((*count, *threshold)),
        }
    }
}

impl ToString for AllocationDetails {
    fn to_string(&self) -> String {
        match self {
            Described(description)      => description.clone(),
            Omitted((count, threshold)) => {
                let plural = if count > &1 { "s" } else { "" };
                format!("in {} place{}, below massif's threshold ({:.2}%)", count, plural, threshold)
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct MassifLine<'a> {
    pub nb_callers: usize,
    pub bytes: usize,
    pub symbol: Symbol<'a>,
}

impl<'a> MassifLine<'a> {
    fn from_tuple(tuple: MassifLineTuple<'a>) -> Self {
        let (nb_callers, bytes, symbol) = tuple;
        MassifLine { nb_callers, bytes, symbol }
    }
}

named!(massif_line<&str, MassifLine>,
       map!(massif_line_tuple, MassifLine::from_tuple));

type MassifLineTuple<'a> = (usize, usize, Symbol<'a>);

named!(massif_line_tuple<&str, MassifLineTuple>,
       terminated!(tuple!(nb_callers, bytes, symbol),
                   do_parse!(opt!(char!('\r')) >> char!('\n') >> ())));

named!(nb_callers<&str, usize>,
       map_res!(preceded!(space0, terminated!(preceded!(char!('n'), digit1), char!(':'))),
                usize::from_str));

named!(bytes<&str, usize>,
       map_res!(preceded!(space0, digit1),
                usize::from_str));

#[derive(Debug, PartialEq)]
pub(crate) enum Symbol<'a> {
    External(&'a str),
    Internal((&'a str, &'a str)),
    Ignored((usize, f32)),
}
use Symbol::*;

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
        assert_eq!(symbol("(heap allocation functions) malloc/new/new[], --alloc-fns, etc.\n"), Ok(("\n", External("(heap allocation functions) malloc/new/new[], --alloc-fns, etc."))));
        assert_eq!(symbol("0x4E23FC67: std::string::_Rep::_S_create(unsigned long, unsigned long, std::allocator<char> const&) (in libstdc++.so)\n"), Ok(("\n", Internal(("4E23FC67", "std::string::_Rep::_S_create(unsigned long, unsigned long, std::allocator<char> const&) (in libstdc++.so)")))));
        assert_eq!(symbol("in 1 place, below massif's threshold (0.01%)"), Ok(("", Ignored((1, 0.01)))));
        assert_eq!(symbol("in 5 places, below massif's threshold (0.01%)"), Ok(("", Ignored((5, 0.01)))));
        assert_eq!(symbol("in 9570 places, all below massif's threshold (0.01%)\n"),
                   Ok(("\n", Ignored((9570, 0.01)))));
    }

    #[test]
    fn it_parses_lines() {
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
