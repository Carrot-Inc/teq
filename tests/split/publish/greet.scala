// A program for the order in which a build's files reach a page (tests/split.sh): `parts` is
// split per file under --module-per-file parts, `logic` and `text` are shared code, of which
// `logic` takes a def of a per-file module.
package parts

def greet(name: String): String = "hello " + name
