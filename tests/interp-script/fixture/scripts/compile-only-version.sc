//> using file ../tools/src
//> using compileOnly.dep dev.zio::zio:2.1.25
// wrong-version.sc's version as a compile-only dependency, whose jar Scala CLI puts on the program's compile
// classpath: the lock refuses it as a dependency's.
println("not run")
