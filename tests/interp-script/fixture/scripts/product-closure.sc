//> using file ../gamma/src
//> using dep org.scala-lang::scala3-library:3.8.4
// gamma/compile owns the sources and builds on alpha/compile, whose products its classpath holds: the script
// runs with alpha's sources, the configuration's product closure.
println(gamma.Hello.greeting)
