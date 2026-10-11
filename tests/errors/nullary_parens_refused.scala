// A parameterless method that overrides no Java method takes no `()`, as under scalac (the control of
// tests/cases/nullary_over_java_parens).
// expect: 7:11: error: method length in trait List does not take parameters
// expect: 8:11: error: method size in class Plain does not take parameters
class Plain { def size: Int = 4 }
@main def run() =
  println(List(1, 2).length())
  println(new Plain().size())
