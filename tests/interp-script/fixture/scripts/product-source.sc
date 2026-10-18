//> using file ../zeta/src
//> using dep org.scala-lang::scala3-library:3.8.4
// zeta/compile owns the sources and builds on epsilon/compile, whose `-source:3.3` the lock records: the script
// runs with epsilon's sources typed at 3.3, as the build compiled the product its classpath holds.
println(zeta.Hello.greeting)
