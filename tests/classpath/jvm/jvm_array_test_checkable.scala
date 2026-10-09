// jars: scala-library
// std: lean scala-library
// teq: --werror
// A singleton's array and an array whose element's type arguments are wildcards are
// checkable tests, as under scalac, and the test of a singleton's array looks at the class
// its value has.
val x: String = "x"
def h(a: Any): Boolean = a match
  case _: Array[x.type] => true
  case _ => false
def k(a: Any): Boolean = a match
  case _: Array[List[? <: String]] => true
  case _ => false
def m(a: Any): Boolean = a match
  case _: Array[Array[? <: AnyRef]] => true
  case _ => false

@main def run(): Unit =
  println(h(Array("y")).toString + k(Array(List("a"))) + m(Array(Array("a"))) + h(Array(1)) + k(Array(1)))
