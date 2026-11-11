// jars: quotenest-lib
// A library macro's nested type pattern binding a variable of its enclosing pattern's name (`case '[t] => .. case
// '[t] =>`): the inner `t` is a fresh variable, which any type matches.
import qnest.*

@main def run(): Unit =
  println(Fresh.result)
