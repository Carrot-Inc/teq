package ot

// A transparent method of a file's top level, which the other file's body expands: its
// `INLINED`'s origin record names the file's `$package` object and the definition's source.
transparent inline def inc(inline x: Int): Int = x + 1
