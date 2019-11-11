use std::error;
use std::fmt;
use std::iter::FromIterator;
use std::str::FromStr;

use nom::character::streaming::{space0, space1, digit1, hex_digit1, line_ending, not_line_ending};
use nom::number::streaming::float;

use crate::{Call, Sample, CallerTree, Attributes, Snapshot, SnapshotId};

named!(pub massif<&str, (Attributes, Vec<Snapshot>)>,
       tuple!(massif_header, many0!(complete!(massif_snapshot))));

// ========== HEADER ==========

named!(massif_header<&str, Attributes>,
       call!(massif_header_attributes));

named!(massif_header_attributes<&str, Attributes>,
       map!(many0!(complete!(massif_header_attribute)), Attributes::from_iter));

named!(massif_header_attribute<&str, (String, String)>,
       do_parse!(
           key: take_till!(|c| c == ':' || c == '\r' || c == '\n')  >> tag!(": ") >>
           value: not_line_ending >> line_ending >>
           (key.to_string(), value.to_string())));

// ========== SNAPSHOT ==========

named!(massif_snapshot<&str, Snapshot>,
       do_parse!(
           id: massif_snapshot_id                 >>
           attributes: massif_snapshot_attributes >>
           tree: massif_tree                      >>
           (Snapshot { id, attributes, tree })));

named!(massif_snapshot_id<&str, SnapshotId>,
       delimited!(massif_snapshot_separator,
           map_res!(massif_snapshot_id_attribute, FromStr::from_str),
           massif_snapshot_separator));

named!(massif_snapshot_separator<&str, ()>,
       do_parse!(space0 >> char!('#') >> many1!(char!('-')) >> line_ending >> (())));

named!(massif_snapshot_id_attribute<&str, &str>,
    do_parse!(
        tag!("snapshot") >> char!('=')  >>
        value: digit1    >> line_ending >>
        (value)));

named!(massif_snapshot_attributes<&str, Attributes>,
       map!(many0!(complete!(massif_snapshot_attribute)), Attributes::from_iter));

named!(massif_snapshot_attribute<&str, (String, String)>,
       do_parse!(
           key: take_till!(|c| c == '=' || c == '\r' || c == '\n')  >> char!('=') >>
           value: not_line_ending >> line_ending >>
           (key.to_string(), value.to_string())));

// ========== TREE & SAMPLES ==========

named!(pub massif_tree<&str, CallerTree>,
       do_parse!(
           sample: massif_sample                        >>
           callers: many_m_n!(0, sample.1, massif_tree) >>
           (CallerTree { sample: sample.0, callers })));

named!(massif_sample<&str, (Sample, usize)>,
       do_parse!(
           space0                                                           >>
           nb_callers: delimited!(char!('n'), positive_integer, char!(':')) >>
           space1                                                           >>
           bytes: positive_integer                                          >>
           space1                                                           >>
           call: massif_call                                                >>
           space0                                                           >>
           line_ending                                                      >>
           (Sample { bytes, call }, nb_callers)));

named!(massif_call<&str, Call>,
       alt!(map!(massif_ignored_call,
                |(count, threshold)| Call::Ignored(count, threshold))
            |
            map!(massif_sampled_call,
                |(address, description)| Call::Sampled(address, description.to_string()))));

named!(massif_sampled_call<&str, (Option<usize>, &str)>,
       do_parse!(
           address: opt!(terminated!(hex_address, char!(':'))) >> space0 >>
           description: not_line_ending                                  >>
           (address, description)));

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

// ========== MISC ==========

named!(hex_address<&str, usize>,
       map_res!(preceded!(tag!("0x"), hex_digit1),
                decode_hex_address));

fn decode_hex_address(data: &str) -> Result<usize, HexDecodeError> {
    let mut bytes = [0u8; 8];

    if data.len() / 2 > bytes.len() {
        return Err(HexDecodeError::InvalidStringLength);
    }

    fn val(c: char, index: usize) -> Result<u8, HexDecodeError> {
        if !c.is_ascii() {
            return Err(HexDecodeError::InvalidHexCharacter { c, index });
        }

        let b = c as u8;
        match b {
            b'A'..=b'F' => Ok(b - b'A' + 10),
            b'a'..=b'f' => Ok(b - b'a' + 10),
            b'0'..=b'9' => Ok(b - b'0'),
            _ => Err(HexDecodeError::InvalidHexCharacter { c, index }),
        }
    }

    let pairs = data.chars().rev().step_by(2).zip(data.chars().rev().skip(1).step_by(2));

    for (i, (lo, hi)) in pairs.enumerate() {
        bytes[i] = val(hi, 2 * i + 1)? << 4 | val(lo, 2 * i)?;
    }

    if data.len() % 2 != 0 {
        let c = data.chars().next().unwrap();
        bytes[data.len()/2] = val(c, data.len())?;
    }

    Ok(usize::from_le_bytes(bytes))
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum HexDecodeError {
    InvalidHexCharacter { c: char, index: usize },
    InvalidStringLength,
}

impl error::Error for HexDecodeError {
    fn description(&self) -> &str {
        match *self {
            Self::InvalidHexCharacter { .. } => "invalid character",
            Self::InvalidStringLength        => "invalid string length",
        }
    }
}

impl fmt::Display for HexDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::InvalidHexCharacter { c, index } => {
                write!(f, "Invalid character '{}' at position {}", c, index)
            }
            Self::InvalidStringLength => write!(f, "Invalid string length"),
        }
    }
}

named!(positive_integer<&str, usize>, map_res!(digit1, usize::from_str));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_parses_ignored_calls() {
        assert_eq!(massif_ignored_call("in 1 place, below massif's threshold (0.01%)"), Ok(("", (1, 0.01))))
    }

    #[test]
    fn it_parses_calls() {
        use Call::*;
        assert_eq!(massif_call("(heap allocation functions) malloc/new/new[], --alloc-fns, etc.\n").map(|(_, o)| o),
                   Ok(Sampled(None, "(heap allocation functions) malloc/new/new[], --alloc-fns, etc.".to_string())));
        assert_eq!(massif_call("0x4E23FC67: std::string::_Rep::_S_create(unsigned long, unsigned long, std::allocator<char> const&) (in libstdc++.so)\n").map(|(_, o)| o),
                   Ok(Sampled(Some(0x4E23FC67), "std::string::_Rep::_S_create(unsigned long, unsigned long, std::allocator<char> const&) (in libstdc++.so)".to_string())));
        assert_eq!(massif_call("in 1 place, below massif's threshold (0.01%)").map(|(_, o)| o),
                   Ok(Ignored(1, 0.01)));
        assert_eq!(massif_call("in 5 places, below massif's threshold (0.01%)").map(|(_, o)| o),
                   Ok(Ignored(5, 0.01)));
        assert_eq!(massif_call("in 9570 places, all below massif's threshold (0.01%)\n").map(|(_, o)| o),
                   Ok(Ignored(9570, 0.01)));
    }

    #[test]
    fn it_parses_samples() {
        use Call::*;
        assert_eq!(massif_sample("n184: 94985897 (heap allocation functions) malloc/new/new[], --alloc-fns, etc.\n").map(|(_, o)| o),
                   Ok((Sample { bytes: 94985897, call: Sampled(None, "(heap allocation functions) malloc/new/new[], --alloc-fns, etc.".to_string()) }, 184)));
        assert_eq!(massif_sample("n4: 13847645 0x4E23FC67: std::string::_Rep::_S_create(unsigned long, unsigned long, std::allocator<char> const&) (in libstdc++.so)\n").map(|(_, o)| o),
                   Ok((Sample { bytes: 13847645, call: Sampled(Some(0x4E23FC67), "std::string::_Rep::_S_create(unsigned long, unsigned long, std::allocator<char> const&) (in libstdc++.so)".to_string()) }, 4)));
        assert_eq!(massif_sample("n0: 109 in 1 place, below massif's threshold (0.01%)\n").map(|(_, o)| o),
                   Ok((Sample { bytes: 109, call: Ignored(1, 0.01) }, 0)));
        assert_eq!(massif_sample("n0: 1355955 in 9570 places, all below massif's threshold (0.01%)\n").map(|(_, o)| o),
                   Ok((Sample { bytes: 1355955, call: Ignored(9570, 0.01) }, 0)));
    }

    #[test]
    fn it_parses_trees() {
        let tree = "\
        n2: 11592561 0x15266383: leafmost_allocation() (in liballoc.so)\n\
         n0: 11592452 0x4E241956: string_allocations() (in libstrings.so)\n\
         n0: 109 in 1 place, below massif's threshold (0.01%)\n\
        ";

        let expected = CallerTree {
            sample: Sample {
                bytes: 11592561,
                call: Call::Sampled(Some(0x15266383), "leafmost_allocation() (in liballoc.so)".to_string()),
            },
            callers: vec![
                CallerTree {
                    sample: Sample {
                        bytes: 11592452,
                        call: Call::Sampled(Some(0x4E241956), "string_allocations() (in libstrings.so)".to_string()),
                    },
                    callers: vec![],
                },
                CallerTree {
                    sample: Sample {
                        bytes: 109,
                        call: Call::Ignored(1, 0.01),
                    },
                    callers: vec![],
                }
            ],
        };

        assert_eq!(massif_tree(tree).map(|(_, o)| o), Ok(expected));
    }

    #[test]
    fn it_parses_snapshot_attributes() {
        assert_eq!(massif_snapshot_attribute("time=0\n").map(|(_, o)| o), Ok(("time".to_string(), "0".to_string())));
        assert_eq!(massif_snapshot_attribute("mem_heap_extra_B=0\n").map(|(_, o)| o), Ok(("mem_heap_extra_B".to_string(), "0".to_string())));
        assert_eq!(massif_snapshot_attribute("mem_stacks_B=0\n").map(|(_, o)| o), Ok(("mem_stacks_B".to_string(), "0".to_string())));
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
            attributes.insert("time".to_string(), "0".to_string());
            attributes.insert("mem_heap_B".to_string(), "0".to_string());
            attributes.insert("mem_heap_extra_B".to_string(), "0".to_string());
            attributes.insert("mem_stacks_B".to_string(), "0".to_string());
            attributes.insert("heap_tree".to_string(), "detailed".to_string());
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
        attributes.insert("time".to_string(), "0".to_string());
        attributes.insert("mem_heap_B".to_string(), "0".to_string());
        attributes.insert("mem_heap_extra_B".to_string(), "0".to_string());
        attributes.insert("mem_stacks_B".to_string(), "0".to_string());
        attributes.insert("heap_tree".to_string(), "detailed".to_string());
        let attributes = attributes;

        let tree = CallerTree {
            sample: Sample {
                bytes: 0,
                call: Call::Sampled(None, "(heap allocation functions) malloc/new/new[], --alloc-fns, etc.".to_string())
            },
            callers: vec![],
        };

        assert_eq!(massif_snapshot(snapshot).map(|(_, o)| o),
                   Ok(Snapshot { id: 0, attributes, tree }));
    }

    #[test]
    fn it_parses_header_attributes() {
        let header = "\
                     desc: -x --option=42 arg1 arg2\n\
                     cmd: the command-line\n\
                     time_unit: ms\n\
                     ";

        let mut attributes = Attributes::new();
        attributes.insert("desc".to_string(), "-x --option=42 arg1 arg2".to_string());
        attributes.insert("cmd".to_string(), "the command-line".to_string());
        attributes.insert("time_unit".to_string(), "ms".to_string());
        let attributes = attributes;

        assert_eq!(massif_header(header).map(|(_, o)| o), Ok(attributes));
    }

    #[test]
    fn it_parses_the_full_output() {
        let out = "\
                     desc: -x --option=42 arg1 arg2\n\
                     cmd: the command-line\n\
                     time_unit: ms\n\
                     #-----------\n\
                     snapshot=0\n\
                     #-----------\n\
                     time=0\n\
                     heap_tree=detailed\n\
                     n0: 0 (heap allocation functions) malloc/new/new[], --alloc-fns, etc.\n\
                     #-----------\n\
                     snapshot=1\n\
                     #-----------\n\
                     time=0\n\
                     heap_tree=detailed\n\
                     n1: 21 (heap allocation functions) malloc/new/new[], --alloc-fns, etc.\n\
                      n0: 21 0x4E23FC67: allocate_some_memory() (in liberty.so)\n\
                     ";

        let header_attributes = {
            let mut attributes = Attributes::new();
            attributes.insert("desc".to_string(), "-x --option=42 arg1 arg2".to_string());
            attributes.insert("cmd".to_string(), "the command-line".to_string());
            attributes.insert("time_unit".to_string(), "ms".to_string());
            attributes
        };

        let snapshot_attributes = {
            let mut attributes = Attributes::new();
            attributes.insert("heap_tree".to_string(), "detailed".to_string());
            attributes.insert("time".to_string(), "0".to_string());
            attributes
        };

        let snapshot0_tree = CallerTree {
            sample: Sample {
                bytes: 0,
                call: Call::Sampled(None, "(heap allocation functions) malloc/new/new[], --alloc-fns, etc.".to_string())
            },
            callers: vec![],
        };

        let snapshot0 = Snapshot {
            id: 0,
            attributes: snapshot_attributes.clone(),
            tree: snapshot0_tree
        };

        let snapshot1_tree = CallerTree {
            sample: Sample {
                bytes: 21,
                call: Call::Sampled(None, "(heap allocation functions) malloc/new/new[], --alloc-fns, etc.".to_string()),
            },
            callers: vec![
                CallerTree {
                    sample: Sample {
                        bytes: 21,
                        call: Call::Sampled(Some(0x4E23FC67), "allocate_some_memory() (in liberty.so)".to_string()),
                    },
                    callers: vec![],
                }
            ],
        };

        let snapshot1 = Snapshot {
            id: 1,
            attributes: snapshot_attributes,
            tree: snapshot1_tree
        };

        assert_eq!(massif(out).map(|(_, o)| o), Ok((header_attributes, vec![snapshot0, snapshot1])));
    }

    #[test]
    fn it_decodes_hex_addresses() {
        assert_eq!(decode_hex_address("12"), Ok(0x12));
        assert_eq!(decode_hex_address("1234"), Ok(0x1234));
        assert_eq!(decode_hex_address("1"), Ok(0x1));
        assert_eq!(decode_hex_address("123"), Ok(0x123));
        assert_eq!(decode_hex_address("15266383"), Ok(0x15266383));
    }
}
