//! The defects the properties found and that master still has. A run stops at its first
//! failure, so the generators leave out what reaches a known defect, each by the narrow
//! precondition named here; `TEQ_PROP_KNOWN=<name>,<name>` puts triggers back in, which is
//! how a defect is shown again and how its repair is checked.
//!
//! - `macro-val-order`: a macro's run that reads a top-level val of another file is refused
//!   ("not supported yet in a macro: .. has no value to read") when the file that calls the
//!   macro comes before the val's file in the order of the inputs. A session's later builds
//!   accept the same sources, so the session differs from a fresh build too. Left out by
//!   naming the val's file `Base.scala`, which comes before every caller, and out of the order
//!   property by leaving tests/split/retype out of its corpora (its macros read `left.scala`
//!   and `right.scala`).
//! - `macro-reads-edited-file`: an edit of a body in a file whose vals or classes a macro's run
//!   read is typed incrementally, and the expansions that read them are not run again
//!   (68d62cff; known to the retype package, whose branch takes the full path). Left out of the
//!   fixture's machine by taking no site of the files the macros read (`left.scala`,
//!   `right.scala`, `base.scala`); the generated programs' macros read nothing a rule edits.
//! - `file-change-with-edit`: a build that finds a file added or removed types the old text of
//!   the files edited in the same build (68d62cff): after a removal the callers' old text is
//!   refused, after an addition the old text is compiled and written. Left out by making a
//!   file's addition or removal two builds, one for the file and one for the edits of the
//!   others.
//! - `inline-signature-uncalled`: a type that does not resolve in the signature of an inline
//!   method is reported only while something calls the method (scalac reports the type). A check
//!   session keeps the error after the last call is taken out, a fresh build has none. Left
//!   out by the model, which withdraws an inline method's broken signature from the text
//!   while nothing calls the method (`Program::shown_fault`) and shows it otherwise. Asking
//!   for it does not show it: the trigger is an error that stands while the last call is
//!   taken out, and a check session's signature errors go in alone (`signature-error-lost`);
//!   split sessions, where they stand, answer as a fresh build does (1000 histories with it
//!   asked). By hand (a01f2f54): a check session over `object A: inline def f(): Missing = 1`
//!   and a `@main` printing `A.f()` keeps `type Missing not found` after the call is replaced
//!   by a literal, where `teq compiler check` reports nothing.
//! - `signature-error-lost`: in a check session a retype of a file takes the errors of its
//!   signatures and imports out of the answer, which a fresh build has (68d62cff; the second
//!   control, known to the retype package). The retype may be that of a body edit in the file
//!   or of an edit of an inline method the file expands. Left out of check sessions by
//!   letting such an error in alone (`edits::alone`): it is repaired in the next build with
//!   nothing else changed, or it stands as the history's last step. An error that stands
//!   while other edits are made is not generated there: the trigger is any retype of the
//!   file, which a change of what the file calls can bring, and on 68d62cff a session after a
//!   failed build retypes files that call nothing that changed; a generator that kept away
//!   from the file and everything it calls met the defect within 27 histories (the first fix
//!   round). Split sessions have both kinds of error interleaved with every edit.
//! - `macro-under-receiver`: a macro's run inside the expansion of an inline method of an
//!   object or a class evaluates the receiver of the inline call at compile time. An object
//!   whose initialiser has an effect is refused ("println is an effect, which a compile-time
//!   evaluation cannot have") and so is a class with a val that reads a constructor parameter
//!   ("the local this$0 is not in scope"), both in a fresh build, where a session's retype
//!   accepts them or refuses them with another message (68d62cff; scalac compiles them). The
//!   macro may stand in the inline method or in a body that the expansion types on demand.
//!   Left out by giving an inline method of an object or a class a body of literals,
//!   parameters and locals, with no call and no macro (`Place::plain`).
//! - `inline-val-members`: an `inline val` has no members, as an operand too (`inline val top =
//!   3`, then `top + 1`: "value + is not a member of 3"), unless the program writes a literal
//!   type somewhere (68d62cff, b0b4d76c; scalac prints 4). Left out of the generated
//!   expressions' programs by a `val written: 1 = 1` in every program that declares an
//!   `inline val`; every use of one as an operand is a trigger.
//! - `inline-val-float`: an `inline val` of a `Float` literal is refused where a `Float` is
//!   required (`inline val f = 0.0f`, then `takes(f)` for a `def takes(x: Float)`: "type
//!   mismatch: found 0, required Float"; 68d62cff; scalac accepts). Left out by making a
//!   `Float` operand a literal or a `final val`: every `inline val` of a `Float` is a
//!   trigger.
//! - `regex-comments`: the interpreter's comments mode keeps white space and `#` inside a class
//!   (`(?x)[a b]` matches a space), ends a comment at `\n` alone where the JDK ends it at `\r`,
//!   NEL, LS and PS too, and refuses white space where the JDK's parser reads past it: before a
//!   quantifier and inside its braces (`a +`, `a{1, 2}`), after `(` of a group, inside an escape
//!   and a property's name (`\x 4 1`, `\p {L}`); JavaScript reads them as the JDK does
//!   (`tests/cases/regex_comments.scala`). Left out of the generated regular expressions
//!   (`regexes`) by writing white space and comments in comments mode between atoms alone, a
//!   comment ended by `\n`.
//! - `regex-properties`: the interpreter's properties are Rust's (`L` Alphabetic, `Lu`
//!   Uppercase, `Ll` Lowercase, `N` and `Nd` Numeric), `Lt` and a `gc=` value are refused, and
//!   `(?i)` widens none, where the JDK's `Lu`, `Ll` and `Lt` are the three categories together;
//!   JavaScript reads them as the JDK does (`tests/cases/regex_case_properties.scala`). Left out
//!   by naming neither `Lt` nor a `gc=` value, no `Lu` or `Ll` under `(?i)`, and, where a pattern
//!   names a property, no input character on which Rust's properties and the JDK's categories
//!   part (`regexes::INPUT_PROPERTIES`).

pub fn asked(name: &str) -> bool {
    std::env::var("TEQ_PROP_KNOWN").is_ok_and(|asked| asked.split(',').any(|one| one.trim() == name))
}
