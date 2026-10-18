//> using options -source:bogus
//> using dep "dev.zio::zio:2.1.26
// An unterminated quoted coordinate beside a -source no version names: Scala CLI's directive lexer refuses the
// header (status 1) before any of its options reaches the compiler.
println("not run")
