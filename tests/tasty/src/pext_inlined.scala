package fix.pext.inlined

// A package's extension, exported from another package as chimney's `dsl` exports `inlined.into`.
extension (s: String) inline def shout: String = s + "!"
