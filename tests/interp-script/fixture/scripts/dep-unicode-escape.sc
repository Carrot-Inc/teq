//> using dep "dev.zio::zio:2.1.26"
// zio's version written with a unicode escape in a quoted coordinate, which Scala CLI's directive lexer decodes
// to 2.1.26, the version tools/compile resolved: the lock accepts it.
println("ok")
