//> using file ../tools/src
//> using	dep dev.zio::zio:2.1.25
// wrong-version.sc's version with a tab between `using` and `dep`, a blank as a space is to Scala CLI's
// directive lexer: the lock refuses it alike.
println("not run")
