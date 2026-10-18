//> using file ../delta/src
//> using dep org.scala-lang::scala3-library:3.8.4
// delta/compile builds on badlib/compile, whose sources run with the script: badlib's using dep line, a version
// the lock does not hold, is refused as the script's own would be.
println(delta.Uses.name)
