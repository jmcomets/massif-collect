use std::collections::HashMap;
use std::io::{self, BufRead};
use std::iter::FromIterator;
use std::str::FromStr;

use nom::character::streaming::{space0, space1, digit1, hex_digit1, line_ending, not_line_ending};
use nom::number::streaming::float;

use crate::{Call, Allocation, Location};

pub fn read_massif_tree<R: BufRead>(reader: R) -> Iter<R> {
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
        let parsed_line = try_iter!(massif_sample(&self.line)
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
    let call = match symbol {
        Symbol::Sampled(Some(address), _) => Call::Inner(address.to_string()),
        Symbol::Sampled(None, _)          => Call::Leaf,
        Symbol::Ignored(_, _)          => Call::Root,
    };

    use Location::*;
    let location = match symbol {
        Symbol::Sampled(_, description)        => Described(description.to_string()),
        Symbol::Ignored(count, threshold) => Omitted((count, threshold)),
    };

    (call, location)
}

named!(massif_header<&str, Attributes>,
       call!(massif_header_attributes));

named!(massif_header_attributes<&str, Attributes>,
       map!(many0!(complete!(massif_header_attribute)), Attributes::from_iter));

named!(massif_header_attribute<&str, (&str, &str)>,
       do_parse!(
           key: take_until!(":")  >> tag!(": ")  >>
           value: not_line_ending >> line_ending >>
           (key, value)));

named!(massif_snapshot<&str, (SnapshotId, Attributes, Tree)>,
       do_parse!(
           id: snapshot_id >>
           attributes: massif_snapshot_attributes >>
           tree: massif_tree >>
           (id, attributes, tree)));

#[allow(dead_code)]
type SnapshotId = usize;

named!(snapshot_id<&str, SnapshotId>,
       delimited!(snapshot_separator,
           map_res!(snapshot_attribute, FromStr::from_str),
           snapshot_separator));

named!(snapshot_separator<&str, ()>,
       do_parse!(space0 >> char!('#') >> many1!(char!('-')) >> line_ending >> (())));

// TODO factorize this parser with the `attribute` parser
named!(snapshot_attribute<&str, &str>,
    do_parse!(
        tag!("snapshot") >> char!('=')  >>
        value: digit1    >> line_ending >>
        (value)));

#[allow(dead_code)]
type Attributes<'a> = HashMap<&'a str, &'a str>;

named!(massif_snapshot_attributes<&str, Attributes>,
       map!(many0!(complete!(massif_snapshot_attribute)), Attributes::from_iter));

named!(massif_snapshot_attribute<&str, (&str, &str)>,
       do_parse!(
           key: take_until!("=")  >> char!('=')  >>
           value: not_line_ending >> line_ending >>
           (key, value)));

#[derive(Debug, PartialEq)]
struct Tree<'a> {
    sample: Sample<'a>,
    callers: Vec<Tree<'a>>,
}

named!(massif_tree<&str, Tree>,
       do_parse!(
           sample: massif_sample                                   >>
           callers: many_m_n!(0, sample.nb_callers, massif_tree) >>
           (Tree { sample, callers })));

#[derive(Debug, PartialEq)]
pub struct Sample<'a> {
    nb_callers: usize,
    bytes: usize,
    symbol: Symbol<'a>,
}

#[derive(Debug, PartialEq)]
pub enum Symbol<'a> {
    Sampled(Option<&'a str>, &'a str),
    Ignored(usize, f32),
}

named!(massif_sample<&str, Sample>,
       do_parse!(
           space0                                                           >>
           nb_callers: delimited!(char!('n'), positive_integer, char!(':')) >>
           space1                                                           >>
           bytes: positive_integer                                          >>
           space1                                                           >>
           symbol: massif_symbol                                            >>
           space0                                                           >>
           line_ending                                                      >>
           (Sample { nb_callers, bytes, symbol })));

named!(massif_symbol<&str, Symbol>,
       alt!(map!(massif_ignored_call,
                |(count, threshold)| Symbol::Ignored(count, threshold))
            |
            map!(massif_sampled_call,
                |(address, description)| Symbol::Sampled(address, description))));

named!(massif_ignored_call<&str, (usize, f32)>,
       do_parse!(
           tag!("in ")                         >>
           nb_places: positive_integer         >>
           tag!(" place") >> opt!(char!('s'))  >>
           tag!(", ") >> opt!(tag!("all "))    >>
           tag!("below massif's threshold (")  >>
           threshold: float                    >>
           tag!("%)")                          >>
           (nb_places, threshold)));

named!(massif_sampled_call<&str, (Option<&str>, &str)>,
       do_parse!(
           address: opt!(terminated!(hex_address, char!(':'))) >> space0 >>
           description: not_line_ending                                  >>
           (address, description)));

named!(hex_address<&str, &str>,
       preceded!(tag!("0x"), hex_digit1));

named!(positive_integer<&str, usize>, map_res!(digit1, usize::from_str));

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn it_parses_ignored_calls() {
        assert_eq!(massif_ignored_call("in 1 place, below massif's threshold (0.01%)"), Ok(("", (1, 0.01))))
    }

    #[test]
    fn it_parses_symbols() {
        use Symbol::*;
        assert_eq!(massif_symbol("(heap allocation functions) malloc/new/new[], --alloc-fns, etc.\n").map(|(_, o)| o),
                   Ok(Sampled(None, "(heap allocation functions) malloc/new/new[], --alloc-fns, etc.")));
        assert_eq!(massif_symbol("0x4E23FC67: std::string::_Rep::_S_create(unsigned long, unsigned long, std::allocator<char> const&) (in libstdc++.so)\n").map(|(_, o)| o),
                   Ok(Sampled(Some("4E23FC67"), "std::string::_Rep::_S_create(unsigned long, unsigned long, std::allocator<char> const&) (in libstdc++.so)")));
        assert_eq!(massif_symbol("in 1 place, below massif's threshold (0.01%)").map(|(_, o)| o),
                   Ok(Ignored(1, 0.01)));
        assert_eq!(massif_symbol("in 5 places, below massif's threshold (0.01%)").map(|(_, o)| o),
                   Ok(Ignored(5, 0.01)));
        assert_eq!(massif_symbol("in 9570 places, all below massif's threshold (0.01%)\n").map(|(_, o)| o),
                   Ok(Ignored(9570, 0.01)));
    }

    #[test]
    fn it_parses_samples() {
        use Symbol::*;
        assert_eq!(massif_sample("n184: 94985897 (heap allocation functions) malloc/new/new[], --alloc-fns, etc.\n").map(|(_, o)| o),
                   Ok(Sample { nb_callers: 184, bytes: 94985897, symbol: Sampled(None, "(heap allocation functions) malloc/new/new[], --alloc-fns, etc.") }));
        assert_eq!(massif_sample("n4: 13847645 0x4E23FC67: std::string::_Rep::_S_create(unsigned long, unsigned long, std::allocator<char> const&) (in libstdc++.so)\n").map(|(_, o)| o),
                   Ok(Sample { nb_callers: 4, bytes: 13847645, symbol: Sampled(Some("4E23FC67"), "std::string::_Rep::_S_create(unsigned long, unsigned long, std::allocator<char> const&) (in libstdc++.so)") }));
        assert_eq!(massif_sample("n0: 109 in 1 place, below massif's threshold (0.01%)\n").map(|(_, o)| o),
                   Ok(Sample { nb_callers: 0, bytes: 109, symbol: Ignored(1, 0.01) }));
        assert_eq!(massif_sample("n0: 1355955 in 9570 places, all below massif's threshold (0.01%)\n").map(|(_, o)| o),
                   Ok(Sample { nb_callers: 0, bytes: 1355955, symbol: Ignored(9570, 0.01) }));
    }

    #[test]
    fn it_parses_trees() {
        // note: this smoke test may not be 100% accurate
        let tree = "\
        n2: 11592561 0x15266383: char* std::string::_S_construct<char const*>(char const*, char const*, std::allocator<char> const&, std::forward_iterator_tag) (in liblog4cxx.so)\n\
         n0: 11592452 0x4E241956: std::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string(char const*, std::allocator<char> const&) (in libstdc++.so)\n\
         n0: 109 in 1 place, below massif's threshold (0.01%)\n\
        n0: 42 in 5 places, below massif's threshold (0.01%)\n\
        ";

        use Location::*;
        let root1 = Allocation::new(11592561, Described("char* std::string::_S_construct<char const*>(char const*, char const*, std::allocator<char> const&, std::forward_iterator_tag) (in liblog4cxx.so)".to_string()));
        let child1 = Allocation::new(11592452, Described("std::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string(char const*, std::allocator<char> const&) (in libstdc++.so)".to_string()));
        let child2 = Allocation::new(109, Omitted((1, 0.01)));
        let root2 = Allocation::new(42, Omitted((5, 0.01)));

        use Call::*;
        let reader = BufReader::new(tree.as_bytes());
        let mut calls = read_massif_tree(reader).map(|result| result.map_err(|e| format!("{:?}", e)));
        assert_eq!(calls.next(), Some(Ok((Inner("15266383".to_string()), None, root1))));
        assert_eq!(calls.next(), Some(Ok((Inner("4E241956".to_string()), Some(Inner("15266383".to_string())), child1))));
        assert_eq!(calls.next(), Some(Ok((Root, Some(Inner("15266383".to_string())), child2))));
        assert_eq!(calls.next(), Some(Ok((Root, None, root2))));
        assert_eq!(calls.next(), None);
    }

    #[test]
    fn it_parses_snapshot_attributes() {
        assert_eq!(massif_snapshot_attribute("time=0\n").map(|(_, o)| o), Ok(("time", "0")));
        assert_eq!(massif_snapshot_attribute("mem_heap_extra_B=0\n").map(|(_, o)| o), Ok(("mem_heap_extra_B", "0")));
        assert_eq!(massif_snapshot_attribute("mem_stacks_B=0\n").map(|(_, o)| o), Ok(("mem_stacks_B", "0")));
    }

    #[test]
    fn it_parses_many_snapshot_attributes() {
        let attributes = "time=0\n\
                          mem_heap_B=0\n\
                          mem_heap_extra_B=0\n\
                          mem_stacks_B=0\n\
                          heap_tree=detailed\n\
                          ";

        let expected = {
            let mut attributes = Attributes::new();
            attributes.insert("time", "0");
            attributes.insert("mem_heap_B", "0");
            attributes.insert("mem_heap_extra_B", "0");
            attributes.insert("mem_stacks_B", "0");
            attributes.insert("heap_tree", "detailed");
            attributes
        };

        assert_eq!(massif_snapshot_attributes(attributes).map(|(_, o)| o), Ok(expected));
    }

    #[test]
    fn it_parses_snapshots() {
        let snapshot = "\
        #-----------\n\
        snapshot=0\n\
        #-----------\n\
        time=0\n\
        mem_heap_B=0\n\
        mem_heap_extra_B=0\n\
        mem_stacks_B=0\n\
        heap_tree=detailed\n\
        n0: 0 (heap allocation functions) malloc/new/new[], --alloc-fns, etc.\n\
        ";

        let mut attributes = Attributes::new();
        attributes.insert("time", "0");
        attributes.insert("mem_heap_B", "0");
        attributes.insert("mem_heap_extra_B", "0");
        attributes.insert("mem_stacks_B", "0");
        attributes.insert("heap_tree", "detailed");
        let attributes = attributes;

        let tree = Tree {
            sample: Sample {
                nb_callers: 0,
                bytes: 0,
                symbol: Symbol::Sampled(None, "(heap allocation functions) malloc/new/new[], --alloc-fns, etc.")
            },
            callers: vec![],
        };

        assert_eq!(massif_snapshot(snapshot).map(|(_, o)| o),
                   Ok((0, attributes, tree)));
    }

    #[test]
    fn it_parses_header_attributes() {
        let header = "\
                     desc: -x --option=42 arg1 arg2\n\
                     cmd: the command-line\n\
                     time_unit: ms\n\
                     ";

        let mut attributes = Attributes::new();
        attributes.insert("desc", "-x --option=42 arg1 arg2");
        attributes.insert("cmd", "the command-line");
        attributes.insert("time_unit", "ms");
        let attributes = attributes;

        assert_eq!(massif_header(header).map(|(_, o)| o), Ok(attributes));
    }
}
