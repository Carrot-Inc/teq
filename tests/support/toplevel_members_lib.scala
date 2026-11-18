// Members a package declares at its top level, which scalac puts in the file's package object: an extension, an
// anonymous given and a named one.
package toplevellib

extension (s: String) def shout: String = s.toUpperCase + "!"

given Int = 42

given number: Long = 7L
