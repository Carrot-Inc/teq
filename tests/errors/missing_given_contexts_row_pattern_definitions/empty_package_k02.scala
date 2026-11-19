val (r02, s02) = ({ def call(using need: Numeric[String]): Int = 1; call }, 2)
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: empty_package_k02.scala:1:73: error: No given instance of type Numeric[String] was found for parameter need of method call
// teq: --std=scala-library
// expect: 12 errors found
// A row: teq refuses a pattern definition at a package's top level, in a package object and in a given's body ("pattern definitions are only supported inside blocks"), where scalac 3.8.4 takes it; the expectations are scalac's lines.
