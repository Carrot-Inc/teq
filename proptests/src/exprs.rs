//! Generated expressions for the property that the targets agree: arithmetic of the numeric
//! types with overflow, division and shifts, comparisons and conversions, `Char` arithmetic,
//! strings over code points outside the basic plane, conditionals and matches over the results,
//! and regular expressions (`regexes`), one case in eight or, with `TEQ_PROP_EXPRS` `regex`
//! (`plain`), every case (none).
//!
//! Every expression is rendered twice: with its operands as the typer can fold them (a literal,
//! a `final val`, an `inline val`) and as it cannot (a parameter, a `var`). The two have one
//! value, so a folder that differs from the run time shows on every target by itself.
//!
//! Left out, as known differences from scalac: how a `Double` or a `Float` prints on
//! JavaScript (its lines are compared between the JVM, the interpreter and scalac, and on
//! JavaScript between the two forms), hashes, `split`, values typed `Any`, lone surrogates in
//! what is printed (a string prints as its code units), `Math.abs` of a `Float` and of
//! -2147483648.0. A chain of `+` has no operand with an effect. The defects of `known` are kept
//! out at the operation that meets them, by a guard in the program's text around that
//! operation alone (`render`, `legs`, `program`).

use hegel::generators as gs;
use hegel::TestCase;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum T {
    Byte,
    Short,
    Char,
    Int,
    Long,
    Float,
    Double,
    Bool,
    Str,
}

impl T {
    pub fn name(self) -> &'static str {
        match self {
            T::Byte => "Byte",
            T::Short => "Short",
            T::Char => "Char",
            T::Int => "Int",
            T::Long => "Long",
            T::Float => "Float",
            T::Double => "Double",
            T::Bool => "Boolean",
            T::Str => "String",
        }
    }

    fn numeric(self) -> bool {
        !matches!(self, T::Bool | T::Str)
    }

    fn conversion(self) -> &'static str {
        match self {
            T::Byte => "toByte",
            T::Short => "toShort",
            T::Char => "toChar",
            T::Int => "toInt",
            T::Long => "toLong",
            T::Float => "toFloat",
            T::Double => "toDouble",
            T::Bool | T::Str => unreachable!(),
        }
    }
}

fn promoted(a: T, b: T) -> T {
    if a == T::Double || b == T::Double {
        T::Double
    } else if a == T::Float || b == T::Float {
        T::Float
    } else if a == T::Long || b == T::Long {
        T::Long
    } else {
        T::Int
    }
}

#[derive(Clone, Debug)]
pub enum E {
    /// An operand: its literal, how the folded form names it (0 the literal, 1 a `final val`,
    /// 2 an `inline val`) and how the other form does (0 a parameter, 1 a `var`).
    Leaf(T, String, u8, u8),
    Unary(&'static str, Box<E>),
    Binary(&'static str, Box<E>, Box<E>),
    Method(&'static str, Box<E>, Vec<E>),
    Static(&'static str, Vec<E>),
    If(Box<E>, Box<E>, Box<E>),
    /// A match with two literal cases, a guard and a default: the scrutinee, the patterns, the
    /// guard's text over `n`, and the four results.
    Match(Box<E>, [String; 2], &'static str, Vec<E>),
    /// A regular expression's operation (`regexes::operation`), its pattern and its input.
    Regex(u8, Box<E>, Box<E>),
}

#[derive(Clone, Debug)]
pub struct Case {
    pub of: T,
    pub e: E,
}

const INTS: [&str; 22] = [
    "0", "1", "-1", "2", "3", "7", "-7", "31", "32", "33", "63", "64", "65", "255", "65535", "65536", "2147483647", "-2147483648",
    "2147483646", "-2147483647", "1000000007", "46341",
];
const LONGS: [&str; 18] = [
    "0L", "1L", "-1L", "2L", "31L", "63L", "64L", "65L", "2147483647L", "2147483648L", "-2147483649L", "4294967296L",
    "9007199254740992L", "9007199254740993L", "-9007199254740993L", "9223372036854775807L", "-9223372036854775808L", "3037000500L",
];
const CHARS: [&str; 8] = ["'a'", "'A'", "'0'", "' '", "'z'", "'\\u00e9'", "'\\u0000'", "'\\uffff'"];
const BYTES: [&str; 5] = ["0", "1", "-1", "127", "-128"];
const SHORTS: [&str; 5] = ["0", "1", "-1", "32767", "-32768"];
const DOUBLES: [&str; 22] = [
    "0.0", "-0.0", "1.0", "-1.0", "0.5", "1.5", "2.5", "-2.5", "0.1", "1.0E21", "1.0E-7", "1.0E7", "9999999.0", "123456789.125",
    "2147483648.0", "-2147483649.0", "9.223372036854776E18", "1.7976931348623157E308", "4.9E-324", "Double.NaN",
    "Double.PositiveInfinity", "Double.NegativeInfinity",
];
const FLOATS: [&str; 14] = [
    "0.0f", "-0.0f", "1.0f", "1.5f", "-2.5f", "0.1f", "1.0E10f", "16777216.0f", "16777217.0f", "3.4028235E38f", "1.4E-45f", "Float.NaN",
    "Float.PositiveInfinity", "Float.NegativeInfinity",
];
const TEXTS: [&str; 14] = [
    "\"\"", "\"a\"", "\"abc\"", "\"Hello\"", "\"\\u00e9\"", "\"\\ud834\\udd1e\"", "\"a\\ud834\\udd1eb\"", "\"\\u00df\"", "\"  x \"", "\"0\"", "\"-12\"",
    "\"2147483648\"", "\"\\u03a9\\u03c9\"", "\"ab\\u00e9cd\"",
];

fn index(tc: &TestCase, count: usize) -> usize {
    tc.draw_silent(gs::integers::<usize>().min_value(0).max_value(count - 1))
}

/// A string and indices into it, any of them: `render` guards the operation.
fn indexed(tc: &TestCase, depth: usize, count: usize) -> (Box<E>, Vec<E>) {
    let any = |tc: &TestCase| E::Leaf(T::Int, ["0", "1", "2", "3", "5", "-1"][index(tc, 6)].to_string(), index(tc, 3) as u8, index(tc, 2) as u8);
    (boxed(tc, T::Str, depth), (0..count).map(|_| any(tc)).collect())
}

fn leaf(tc: &TestCase, of: T) -> E {
    let literal = match of {
        T::Byte => BYTES[index(tc, BYTES.len())].to_string(),
        T::Short => SHORTS[index(tc, SHORTS.len())].to_string(),
        T::Char => CHARS[index(tc, CHARS.len())].to_string(),
        T::Int => match index(tc, INTS.len() + 1) {
            0 => tc.draw_silent(gs::integers::<i32>()).to_string(),
            n => INTS[n - 1].to_string(),
        },
        T::Long => match index(tc, LONGS.len() + 1) {
            0 => format!("{}L", tc.draw_silent(gs::integers::<i64>())),
            n => LONGS[n - 1].to_string(),
        },
        T::Float => FLOATS[index(tc, FLOATS.len())].to_string(),
        T::Double => DOUBLES[index(tc, DOUBLES.len())].to_string(),
        T::Bool => ["false", "true"][index(tc, 2)].to_string(),
        T::Str => TEXTS[index(tc, TEXTS.len())].to_string(),
    };
    // A name of `Double` or `Float` is no literal and a byte or a short has none, so no
    // `inline val` takes them.
    let float = of == T::Float && !crate::known::asked("inline-val-float");
    let named = float || matches!(of, T::Byte | T::Short) || literal.chars().next().is_some_and(|c| c.is_alphabetic() && of != T::Bool);
    let folded = index(tc, if named { 2 } else { 3 }) as u8;
    E::Leaf(of, literal, folded, index(tc, 2) as u8)
}

const NUMERIC: [T; 7] = [T::Int, T::Long, T::Char, T::Double, T::Float, T::Byte, T::Short];
const INTEGRAL: [T; 5] = [T::Int, T::Long, T::Char, T::Byte, T::Short];

pub fn type_of(e: &E) -> T {
    match e {
        E::Leaf(of, ..) => *of,
        E::Unary("!", _) => T::Bool,
        E::Unary(_, a) => promoted(type_of(a), T::Int),
        E::Binary(op, a, b) => match *op {
            "<" | "<=" | ">" | ">=" | "==" | "!=" | "&&" | "||" => T::Bool,
            "<<" | ">>" | ">>>" => promoted(type_of(a), T::Int),
            "+" if type_of(a) == T::Str => T::Str,
            _ if type_of(a) == T::Bool => T::Bool,
            _ => promoted(type_of(a), type_of(b)),
        },
        E::Method(name, a, _) => match *name {
            "toByte" => T::Byte,
            "toShort" => T::Short,
            "toChar" | "charAt" => T::Char,
            "toInt" | "length" | "codePointAt" | "indexOf" | "lastIndexOf" | "compareTo" => T::Int,
            "toLong" => T::Long,
            "toFloat" => T::Float,
            "toDouble" => T::Double,
            "startsWith" | "endsWith" | "contains" | "isEmpty" | "isNaN" => T::Bool,
            "substring" | "toUpperCase" | "toLowerCase" | "trim" | "*" => T::Str,
            _ => type_of(a),
        },
        E::Static(_, args) => type_of(&args[0]),
        E::If(_, a, _) => type_of(a),
        E::Match(_, _, _, results) => type_of(&results[0]),
        E::Regex(..) => T::Str,
    }
}

/// An expression of the given type: one of another numeric type is converted.
fn of(tc: &TestCase, wanted: T, depth: usize) -> E {
    let e = expression(tc, wanted, depth);
    let found = type_of(&e);
    if found == wanted {
        e
    } else {
        assert!(found.numeric() && wanted.numeric(), "{found:?} where {wanted:?} is wanted: {e:?}");
        E::Method(wanted.conversion(), Box::new(e), Vec::new())
    }
}

fn boxed(tc: &TestCase, wanted: T, depth: usize) -> Box<E> {
    Box::new(of(tc, wanted, depth))
}

fn some(tc: &TestCase, types: &[T]) -> T {
    types[index(tc, types.len())]
}

/// An expression whose type is `wanted`, or for a numeric `wanted` any numeric type.
fn expression(tc: &TestCase, wanted: T, depth: usize) -> E {
    if depth == 0 || index(tc, 5) == 0 {
        return leaf(tc, wanted);
    }
    let d = depth - 1;
    let small = |tc: &TestCase| E::Leaf(T::Int, ["0", "1", "2", "3", "5", "-1"][index(tc, 6)].to_string(), index(tc, 3) as u8, index(tc, 2) as u8);
    match wanted {
        T::Bool => match index(tc, 7) {
            0 => E::Binary(["&&", "||", "&", "|", "^"][index(tc, 5)], boxed(tc, T::Bool, d), boxed(tc, T::Bool, d)),
            1 => E::Unary("!", boxed(tc, T::Bool, d)),
            2 => E::Binary(["==", "!="][index(tc, 2)], boxed(tc, T::Str, d), boxed(tc, T::Str, d)),
            3 => E::Method(["startsWith", "endsWith", "contains"][index(tc, 3)], boxed(tc, T::Str, d), vec![of(tc, T::Str, 0)]),
            4 => E::Method("isNaN", boxed(tc, some(tc, &[T::Double, T::Float]), d), Vec::new()),
            _ => {
                let op = ["<", "<=", ">", ">=", "==", "!="][index(tc, 6)];
                E::Binary(op, boxed(tc, some(tc, &NUMERIC), d), boxed(tc, some(tc, &NUMERIC), d))
            }
        },
        T::Str => match index(tc, 6) {
            0 => {
                let joined = some(tc, &[T::Str, T::Int, T::Long, T::Bool, T::Byte, T::Short]);
                E::Binary("+", boxed(tc, T::Str, d), boxed(tc, joined, d))
            }
            1 => E::Binary("+", boxed(tc, T::Str, d), Box::new(leaf(tc, T::Char))),
            2 => {
                let (text, indices) = indexed(tc, d, 2);
                E::Method("substring", text, indices)
            }
            3 => E::Method(["toUpperCase", "toLowerCase", "trim"][index(tc, 3)], boxed(tc, T::Str, d), Vec::new()),
            4 => E::Method("*", boxed(tc, T::Str, d), vec![small(tc)]),
            _ => conditional(tc, T::Str, d),
        },
        _ => match index(tc, 12) {
            0 | 1 => {
                let op = ["+", "-", "*", "/", "%"][index(tc, 5)];
                E::Binary(op, boxed(tc, some(tc, &NUMERIC), d), boxed(tc, some(tc, &NUMERIC), d))
            }
            2 => E::Binary(["&", "|", "^"][index(tc, 3)], boxed(tc, some(tc, &INTEGRAL), d), boxed(tc, some(tc, &INTEGRAL), d)),
            3 | 4 => {
                let count = some(tc, &[T::Int, T::Int, T::Byte, T::Char]);
                E::Binary(["<<", ">>", ">>>"][index(tc, 3)], boxed(tc, some(tc, &INTEGRAL), d), boxed(tc, count, d))
            }
            5 => E::Unary("-", boxed(tc, some(tc, &NUMERIC), d)),
            6 => E::Unary("~", boxed(tc, some(tc, &INTEGRAL), d)),
            7 => E::Method(some(tc, &NUMERIC).conversion(), boxed(tc, some(tc, &NUMERIC), d), Vec::new()),
            8 => match index(tc, 4) {
                0 => E::Method("length", boxed(tc, T::Str, d), Vec::new()),
                1 => {
                    let (text, indices) = indexed(tc, d, 1);
                    E::Method(["charAt", "codePointAt"][index(tc, 2)], text, indices)
                }
                2 => E::Method(["indexOf", "lastIndexOf"][index(tc, 2)], boxed(tc, T::Str, d), vec![of(tc, T::Str, 0)]),
                _ => E::Method("toInt", boxed(tc, T::Str, d), Vec::new()),
            },
            9 => {
                let over = some(tc, &[T::Int, T::Long]);
                // teq's `Math.abs` takes no `Float`.
                match index(tc, 3) {
                    0 => E::Static("Math.abs", vec![of(tc, some(tc, &[T::Int, T::Long, T::Double]), d)]),
                    1 => E::Static("Math.floorMod", vec![of(tc, over, d), of(tc, over, d)]),
                    _ => E::Static("Math.floorDiv", vec![of(tc, over, d), of(tc, over, d)]),
                }
            }
            _ => conditional(tc, some(tc, &NUMERIC), d),
        },
    }
}

fn conditional(tc: &TestCase, wanted: T, depth: usize) -> E {
    if index(tc, 2) == 0 {
        return E::If(boxed(tc, T::Bool, depth), boxed(tc, wanted, depth), boxed(tc, wanted, depth));
    }
    let over = some(tc, &[T::Int, T::Char, T::Str, T::Long]);
    let patterns = [leaf(tc, over), leaf(tc, over)].map(|pattern| match pattern {
        E::Leaf(_, literal, ..) => literal,
        _ => unreachable!(),
    });
    let guard = match over {
        T::Str => ["n.length > 2", "n.isEmpty"][index(tc, 2)],
        T::Char => ["n < 'a'", "n.isDigit"][index(tc, 2)],
        _ => ["n < 0", "n % 2 == 0"][index(tc, 2)],
    };
    let results = (0..4).map(|_| of(tc, wanted, 0)).collect();
    E::Match(boxed(tc, over, depth), patterns, guard, results)
}

pub fn case(tc: &TestCase) -> Case {
    let regex = match std::env::var("TEQ_PROP_EXPRS").as_deref() {
        Ok("regex") => true,
        Ok("plain") => false,
        _ => index(tc, 8) == 0,
    };
    if regex {
        return crate::regexes::case(tc);
    }
    let wanted = [T::Int, T::Long, T::Double, T::Str, T::Bool, T::Char, T::Float, T::Byte, T::Short][index(tc, 9)];
    let e = expression(tc, wanted, 3);
    Case { of: type_of(&e), e }
}

/// The operands of one form, as the program declares them.
#[derive(Default)]
struct Names {
    constants: Vec<String>,
    variables: Vec<String>,
    parameters: Vec<(String, String)>,
}

fn typed(of: T, literal: &str) -> String {
    match of {
        T::Byte | T::Short => format!("({literal}: {})", of.name()),
        _ if literal.starts_with('-') => format!("({literal})"),
        _ => literal.to_string(),
    }
}


fn floating(e: &E) -> bool {
    matches!(type_of(e), T::Double | T::Float)
}

fn render(e: &E, folded: bool, prefix: &str, names: &mut Names) -> String {
    let sub = |e: &E, names: &mut Names| render(e, folded, prefix, names);
    match e {
        E::Leaf(of, literal, fold, run) => match (folded, fold, run) {
            (true, 0, _) => typed(*of, literal),
            (true, how, _) => {
                let name = format!("{prefix}c{}", names.constants.len());
                let inline = if *how == 2 { "inline" } else { "final" };
                // A constant has no declared type; a byte or a short has none as a literal.
                let value = if matches!(of, T::Byte | T::Short) { typed(*of, literal) } else { literal.clone() };
                names.constants.push(format!("  {inline} val {name} = {value}"));
                format!("Consts.{name}")
            }
            (false, _, 0) => {
                let name = format!("p{}", names.parameters.len());
                names.parameters.push((format!("{name}: {}", of.name()), typed(*of, literal)));
                name
            }
            (false, _, _) => {
                let name = format!("{prefix}v{}", names.variables.len());
                names.variables.push(format!("  var {name}: {} = {literal}", of.name()));
                format!("Vars.{name}")
            }
        },
        // `-32.toChar` is a negative literal's `toChar`, `-(32.toChar)` the prefix operator's.
        E::Unary(op, a) => format!("({op}({}))", sub(a, names)),
        E::Binary(op, a, b) => {
            let left = sub(a, names);
            format!("({left} {op} {})", sub(b, names))
        }
        E::Method(name, a, args) => {
            let receiver = sub(a, names);
            let args: Vec<String> = args.iter().map(|arg| sub(arg, names)).collect();
            match args.is_empty() {
                true => format!("{receiver}.{name}"),
                false => format!("{receiver}.{name}({})", args.join(", ")),
            }
        }
        // `Math.abs` of a floating operand is not asked of -2147483648.0, whose `abs` is
        // negative on JavaScript, a known difference.
        E::Static("Math.abs", args) if floating(&args[0]) => {
            let operand = sub(&args[0], names);
            format!("(if {operand} == -2147483648.0 then 2147483648.0 else Math.abs({operand}))")
        }
        E::Static(name, args) => {
            let args: Vec<String> = args.iter().map(|arg| sub(arg, names)).collect();
            format!("{name}({})", args.join(", "))
        }
        E::If(c, a, b) => {
            let condition = sub(c, names);
            let then = sub(a, names);
            format!("(if {condition} then {then} else {})", sub(b, names))
        }
        E::Regex(op, pattern, input) => {
            let pattern = sub(pattern, names);
            crate::regexes::operation(*op, &pattern, &sub(input, names))
        }
        E::Match(scrutinee, patterns, guard, results) => {
            let scrutinee = sub(scrutinee, names);
            let results: Vec<String> = results.iter().map(|result| sub(result, names)).collect();
            // Equal patterns would make the second case unreachable, which scalac warns of.
            let second = match patterns[0] == patterns[1] {
                true => String::new(),
                false => format!("case {} => {}; ", patterns[1], results[1]),
            };
            format!(
                "({scrutinee} match {{ case {} => {}; {second}case n if {guard} => {}; case _ => {} }})",
                patterns[0], results[0], results[2], results[3]
            )
        }
    }
}

/// How a value of the type is printed: `v` lines hold on every target, `t` lines hold how a
/// `Double` or a `Float` prints, which JavaScript does in its own way.
fn legs(of: T, value: &str) -> Vec<(&'static str, String)> {
    match of {
        T::Char => vec![("v", format!("{value}.toInt.toString"))],
        T::Str => vec![("v", format!("units({value})"))],
        T::Double | T::Float => {
            let bits = if of == T::Double { "java.lang.Double.doubleToLongBits" } else { "java.lang.Float.floatToIntBits" };
            let shown = format!("{bits}({value}).toString");
            vec![("v", shown), ("t", format!("{value}.toString"))]
        }
        _ => vec![("v", format!("{value}.toString"))],
    }
}

/// The program that prints every case in both forms, one line each: `<case>.<f|r>.<v|t>:<value>`,
/// and the labels of its lines in their order.
pub fn program(cases: &[Case]) -> (String, Vec<String>) {
    let mut names = Names::default();
    let mut defs = String::new();
    let mut main = String::new();
    let mut labels = Vec::new();
    for (n, case) in cases.iter().enumerate() {
        let prefix = format!("k{n}");
        let folded = render(&case.e, true, &prefix, &mut names);
        for (leg, text) in legs(case.of, &format!("({folded})")) {
            main.push_str(&format!("  show(\"{n}.f.{leg}\", {text})\n"));
            labels.push(format!("{n}.f.{leg}"));
        }
        names.parameters.clear();
        let run = render(&case.e, false, &prefix, &mut names);
        let (parameters, arguments): (Vec<String>, Vec<String>) = names.parameters.drain(..).unzip();
        for (leg, text) in legs(case.of, &format!("({run})")) {
            defs.push_str(&format!("def r{n}{leg}({}): String =\n  {text}\n\n", parameters.join(", ")));
            main.push_str(&format!("  show(\"{n}.r.{leg}\", r{n}{leg}({}))\n", arguments.join(", ")));
            labels.push(format!("{n}.r.{leg}"));
        }
    }
    // A literal type written somewhere gives an `inline val` its members (`inline-val-members`).
    let primed = names.constants.iter().any(|line| line.contains("inline val")) && !crate::known::asked("inline-val-members");
    let written = if primed { "\n  val written: 1 = 1" } else { "" };
    let text = format!(
        "object Consts:\n  final val none = 0{written}\n{}\n\nobject Vars:\n  var none: Int = 0\n{}\n\n\
         def units(text: String): String =\n  var out = \"\"\n  var i = 0\n  while i < text.length do\n    out = out + text.charAt(i).toInt + \" \"\n    i += 1\n  out\n\n\
         def show(id: String, text: => String): Unit =\n  val shown =\n    try text\n    catch\n      case e: ArithmeticException => \"ArithmeticException\"\n      \
         case e: IndexOutOfBoundsException => \"IndexOutOfBoundsException\"\n      case e: NumberFormatException => \"NumberFormatException\"\n  println(id + \":\" + shown)\n\n\
         {defs}@main def run(): Unit =\n{main}",
        names.constants.join("\n"),
        names.variables.join("\n"),
    );
    (text, labels)
}

/// One case's two forms as the program has them, for a report.
pub fn shown(case: &Case) -> String {
    let mut names = Names::default();
    let folded = render(&case.e, true, "k", &mut names);
    let run = render(&case.e, false, "k", &mut names);
    let mut lines = vec![format!("folded: {folded}"), format!("at run time: {run}")];
    lines.extend(names.constants.iter().map(|line| format!("  where{}", &line[1..])));
    lines.extend(names.variables.iter().map(|line| format!("  where{}", &line[1..])));
    lines.extend(names.parameters.iter().map(|(parameter, argument)| format!("  where {parameter} is {argument}")));
    lines.join("\n")
}
