//> using file ../tools/src
//> using `dep` dev.zio::zio:2.1.25
// wrong-version.sc's version with its key backtick-quoted, an identifier as Scala CLI's directive lexer reads
// it: the lock refuses it alike.
println("not run")
