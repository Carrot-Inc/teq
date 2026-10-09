//! `teq tasty --trees`: the raw form of a TASTy file, tag by tag, as scalac's `TastyPrinter`
//! shows it: the header, the name table with the kind of each name, the trees with their
//! addresses and lengths, the positions and the attributes. It reads what a writer wrote without
//! interpreting it, which is what comparing two picklers needs.

use super::show::name_kind;
use super::tags::*;
use super::{Reader, TName, TastyFile};

pub fn dump(file: &TastyFile, out: &mut String) {
    let bytes = &file.bytes;
    let mut r = Reader::new(bytes);
    r.pos = 4;
    let (major, minor, experimental) = (r.nat(), r.nat(), r.nat());
    let tooling_len = r.nat() as usize;
    let tooling = String::from_utf8_lossy(&bytes[r.pos..(r.pos + tooling_len).min(bytes.len())]).to_string();
    r.pos += tooling_len;
    let uuid: String = bytes.get(r.pos..r.pos + 16).map_or(String::new(), |u| u.iter().map(|b| format!("{:02x}", b)).collect());
    out.push_str(&format!("Header: version {}.{}.{}, tooling \"{}\", UUID {}\n", major, minor, experimental, tooling, uuid));
    out.push_str(&format!("Names ({}):\n", file.names.len()));
    for (i, n) in file.names.iter().enumerate() {
        out.push_str(&format!("{:>6}: {} {}\n", i, name_kind(n), name_text(file, i as u32)));
    }
    let trees = &bytes[file.asts.clone()];
    out.push_str(&format!("Trees ({} bytes):\n", trees.len()));
    let mut d = Dumper { file, r: Reader::new(trees), out, indent: 0 };
    while !d.r.at_end() {
        let before = d.r.pos;
        d.tree();
        if d.r.pos == before {
            break;
        }
    }
    positions(file, out);
    attributes(file, out);
    super::origins::show(file, out);
}

/// The name with its structure: a signed name shows its signature, a qualified one its parts.
pub fn name_text(file: &TastyFile, n: u32) -> String {
    match file.names.get(n as usize) {
        Some(TName::Signed { original, target, result, params }) => {
            let ps: Vec<String> = params.iter().map(|&p| if p < 0 { format!("[{}]", -p) } else { file.name(p as u32) }).collect();
            let target = target.map_or(String::new(), |t| format!(" @{}", file.name(t)));
            format!("{}({}): {}{}", file.name(*original), ps.join(", "), file.name(*result), target)
        }
        _ => file.name(n),
    }
}

struct Dumper<'a> {
    file: &'a TastyFile,
    r: Reader<'a>,
    out: &'a mut String,
    indent: usize,
}

impl<'a> Dumper<'a> {
    fn line(&mut self, addr: usize, text: &str) {
        self.out.push_str(&format!("{:>6}:{}{}\n", addr, " ".repeat(self.indent), text));
    }

    fn name(&mut self) -> String {
        let n = self.r.nat();
        format!("{} [{}]", n, name_text(self.file, n))
    }

    fn tree(&mut self) {
        let addr = self.r.pos;
        let tag = self.r.byte();
        let tag_name = tag_name(tag);
        if tag >= 128 {
            let end = self.r.end();
            let head = match tag {
                VALDEF | DEFDEF | TYPEDEF | TYPEPARAM | PARAM | BIND => format!("{}({}) {}", tag_name, end - self.r.pos, self.name()),
                REFINEDTYPE | TERMREFIN | TYPEREFIN | SELECTIN => format!("{}({}) {}", tag_name, end - self.r.pos, self.name()),
                RETURN | HOLE => format!("{}({}) {}", tag_name, end - self.r.pos, self.r.nat()),
                PARAMTYPE => {
                    let len = end - self.r.pos;
                    let binder = self.r.nat();
                    let num = self.r.nat();
                    format!("{}({}) binder {} #{}", tag_name, len, binder, num)
                }
                _ => format!("{}({})", tag_name, end - self.r.pos),
            };
            self.line(addr, &head);
            self.indent += 2;
            match tag {
                METHODTYPE | POLYTYPE | TYPELAMBDATYPE => {
                    self.tree();
                    while self.r.pos < end && !is_modifier(self.r.peek()) {
                        self.tree();
                        let n = self.name();
                        let at = self.r.pos;
                        self.line(at, &format!("  name {}", n));
                    }
                }
                _ => {}
            }
            while self.r.pos < end {
                let before = self.r.pos;
                self.tree();
                if self.r.pos == before {
                    break;
                }
            }
            if self.r.pos != end {
                self.line(self.r.pos, &format!("<incomplete read, end {}>", end));
                self.r.pos = end;
            }
            self.indent -= 2;
        } else if tag >= 110 {
            let head = match tag {
                IDENT | IDENTTPT | SELECT | SELECTTPT | TERMREF | TYPEREF | SELFDEF | NAMEDARG => format!("{} {}", tag_name, self.name()),
                _ => format!("{} {}", tag_name, self.r.nat()),
            };
            self.line(addr, &head);
            self.indent += 2;
            self.tree();
            self.indent -= 2;
        } else if tag >= 90 {
            self.line(addr, tag_name);
            self.indent += 2;
            self.tree();
            self.indent -= 2;
        } else if tag >= 60 {
            let text = match tag {
                TERMREFPKG | TYPEREFPKG | STRINGCONST | IMPORTED | RENAMED => format!("{} {}", tag_name, self.name()),
                LONGCONST | DOUBLECONST => format!("{} {}", tag_name, self.r.long_int()),
                BYTECONST | SHORTCONST | INTCONST | FLOATCONST => format!("{} {}", tag_name, self.r.long_int()),
                _ => format!("{} {}", tag_name, self.r.nat()),
            };
            self.line(addr, &text);
        } else {
            self.line(addr, tag_name);
        }
    }
}

fn positions(file: &TastyFile, out: &mut String) {
    let Some(range) = section(file, "Positions") else { return };
    let mut r = Reader::new(&file.bytes[range]);
    let lines = r.nat();
    let sizes: Vec<u32> = (0..lines).map(|_| r.nat()).collect();
    out.push_str(&format!("Positions: {} lines, sizes {:?}\n", lines, sizes));
    let (mut addr, mut start, mut end) = (0i64, 0i64, 0i64);
    while !r.at_end() {
        let header = r.long_int();
        if header == 4 {
            let n = r.long_int();
            out.push_str(&format!("  source {} [{}]\n", n, file.name(n as u32)));
            continue;
        }
        addr += header >> 3;
        if header & 4 != 0 {
            start += r.long_int();
        }
        if header & 2 != 0 {
            end += r.long_int();
        }
        let point = if header & 1 != 0 { format!(" point {}", start + r.long_int()) } else { String::new() };
        out.push_str(&format!("  {}: {} .. {}{}\n", addr, start, end, point));
    }
}

fn attributes(file: &TastyFile, out: &mut String) {
    let Some(range) = section(file, "Attributes") else { return };
    let mut r = Reader::new(&file.bytes[range]);
    let mut shown = Vec::new();
    while !r.at_end() {
        let tag = r.byte();
        let name = match tag {
            1 => "SCALA2STANDARDLIBRARYattr".to_string(),
            2 => "EXPLICITNULLSattr".to_string(),
            3 => "CAPTURECHECKEDattr".to_string(),
            4 => "WITHPUREFUNSattr".to_string(),
            5 => "JAVAattr".to_string(),
            6 => "OUTLINEattr".to_string(),
            129 => {
                let n = r.nat();
                format!("SOURCEFILEattr {} [{}]", n, file.name(n))
            }
            t => format!("<attribute {}>", t),
        };
        shown.push(name);
    }
    out.push_str(&format!("Attributes: {}\n", shown.join(", ")));
}

/// The bytes of the section of that name, after its length.
pub fn section(file: &TastyFile, wanted: &str) -> Option<std::ops::Range<usize>> {
    let bytes = &file.bytes;
    let mut r = Reader::new(bytes);
    r.pos = 4;
    r.nat();
    r.nat();
    r.nat();
    let tooling = r.nat() as usize;
    r.pos += tooling + 16;
    let names_end = r.end();
    r.pos = names_end;
    while !r.at_end() {
        let name = r.nat();
        let end = r.end();
        if file.simple(name) == Some(wanted) {
            return Some(r.pos..end);
        }
        r.pos = end;
    }
    None
}

pub fn tag_name(tag: u8) -> &'static str {
    match tag {
        UNITCONST => "UNITconst",
        FALSECONST => "FALSEconst",
        TRUECONST => "TRUEconst",
        NULLCONST => "NULLconst",
        PRIVATE => "PRIVATE",
        PROTECTED => "PROTECTED",
        ABSTRACT => "ABSTRACT",
        FINAL => "FINAL",
        SEALED => "SEALED",
        CASE => "CASE",
        IMPLICIT => "IMPLICIT",
        LAZY => "LAZY",
        OVERRIDE => "OVERRIDE",
        INLINEPROXY => "INLINEPROXY",
        INLINE => "INLINE",
        STATIC => "STATIC",
        OBJECT => "OBJECT",
        TRAIT => "TRAIT",
        ENUM => "ENUM",
        LOCAL => "LOCAL",
        SYNTHETIC => "SYNTHETIC",
        ARTIFACT => "ARTIFACT",
        MUTABLE => "MUTABLE",
        FIELDACCESSOR => "FIELDaccessor",
        CASEACCESSOR => "CASEaccessor",
        COVARIANT => "COVARIANT",
        CONTRAVARIANT => "CONTRAVARIANT",
        HASDEFAULT => "HASDEFAULT",
        STABLE => "STABLE",
        MACRO => "MACRO",
        ERASED => "ERASED",
        OPAQUE => "OPAQUE",
        EXTENSION => "EXTENSION",
        GIVEN => "GIVEN",
        PARAMSETTER => "PARAMsetter",
        EXPORTED => "EXPORTED",
        OPEN => "OPEN",
        PARAMALIAS => "PARAMalias",
        TRANSPARENT => "TRANSPARENT",
        INFIX => "INFIX",
        INVISIBLE => "INVISIBLE",
        EMPTYCLAUSE => "EMPTYCLAUSE",
        SPLITCLAUSE => "SPLITCLAUSE",
        TRACKED => "TRACKED",
        SUBMATCH => "SUBMATCH",
        INTO => "INTO",
        SHAREDTERM => "SHAREDterm",
        SHAREDTYPE => "SHAREDtype",
        TERMREFDIRECT => "TERMREFdirect",
        TYPEREFDIRECT => "TYPEREFdirect",
        TERMREFPKG => "TERMREFpkg",
        TYPEREFPKG => "TYPEREFpkg",
        RECTHIS => "RECthis",
        BYTECONST => "BYTEconst",
        SHORTCONST => "SHORTconst",
        CHARCONST => "CHARconst",
        INTCONST => "INTconst",
        LONGCONST => "LONGconst",
        FLOATCONST => "FLOATconst",
        DOUBLECONST => "DOUBLEconst",
        STRINGCONST => "STRINGconst",
        IMPORTED => "IMPORTED",
        RENAMED => "RENAMED",
        THIS => "THIS",
        QUALTHIS => "QUALTHIS",
        CLASSCONST => "CLASSconst",
        BYNAMETYPE => "BYNAMEtype",
        BYNAMETPT => "BYNAMEtpt",
        NEW => "NEW",
        THROW => "THROW",
        IMPLICITARG => "IMPLICITarg",
        PRIVATEQUALIFIED => "PRIVATEqualified",
        PROTECTEDQUALIFIED => "PROTECTEDqualified",
        RECTYPE => "RECtype",
        SINGLETONTPT => "SINGLETONtpt",
        BOUNDED => "BOUNDED",
        EXPLICITTPT => "EXPLICITtpt",
        ELIDED => "ELIDED",
        IDENT => "IDENT",
        IDENTTPT => "IDENTtpt",
        SELECT => "SELECT",
        SELECTTPT => "SELECTtpt",
        TERMREFSYMBOL => "TERMREFsymbol",
        TERMREF => "TERMREF",
        TYPEREFSYMBOL => "TYPEREFsymbol",
        TYPEREF => "TYPEREF",
        SELFDEF => "SELFDEF",
        NAMEDARG => "NAMEDARG",
        PACKAGE => "PACKAGE",
        VALDEF => "VALDEF",
        DEFDEF => "DEFDEF",
        TYPEDEF => "TYPEDEF",
        IMPORT => "IMPORT",
        TYPEPARAM => "TYPEPARAM",
        PARAM => "PARAM",
        APPLY => "APPLY",
        TYPEAPPLY => "TYPEAPPLY",
        TYPED => "TYPED",
        ASSIGN => "ASSIGN",
        BLOCK => "BLOCK",
        IF => "IF",
        LAMBDA => "LAMBDA",
        MATCH => "MATCH",
        RETURN => "RETURN",
        WHILE => "WHILE",
        TRY => "TRY",
        INLINED => "INLINED",
        SELECTOUTER => "SELECTouter",
        REPEATED => "REPEATED",
        BIND => "BIND",
        ALTERNATIVE => "ALTERNATIVE",
        UNAPPLY => "UNAPPLY",
        ANNOTATEDTYPE => "ANNOTATEDtype",
        ANNOTATEDTPT => "ANNOTATEDtpt",
        CASEDEF => "CASEDEF",
        TEMPLATE => "TEMPLATE",
        SUPER => "SUPER",
        SUPERTYPE => "SUPERtype",
        REFINEDTYPE => "REFINEDtype",
        REFINEDTPT => "REFINEDtpt",
        APPLIEDTYPE => "APPLIEDtype",
        APPLIEDTPT => "APPLIEDtpt",
        TYPEBOUNDS => "TYPEBOUNDS",
        TYPEBOUNDSTPT => "TYPEBOUNDStpt",
        ANDTYPE => "ANDtype",
        ORTYPE => "ORtype",
        POLYTYPE => "POLYtype",
        TYPELAMBDATYPE => "TYPELAMBDAtype",
        LAMBDATPT => "LAMBDAtpt",
        PARAMTYPE => "PARAMtype",
        ANNOTATION => "ANNOTATION",
        TERMREFIN => "TERMREFin",
        TYPEREFIN => "TYPEREFin",
        SELECTIN => "SELECTin",
        EXPORT => "EXPORT",
        QUOTE => "QUOTE",
        SPLICE => "SPLICE",
        METHODTYPE => "METHODtype",
        APPLYSIGPOLY => "APPLYsigpoly",
        QUOTEPATTERN => "QUOTEPATTERN",
        SPLICEPATTERN => "SPLICEPATTERN",
        MATCHTYPE => "MATCHtype",
        MATCHTPT => "MATCHtpt",
        MATCHCASETYPE => "MATCHCASEtype",
        FLEXIBLETYPE => "FLEXIBLEtype",
        HOLE => "HOLE",
        _ => "<unknown tag>",
    }
}
