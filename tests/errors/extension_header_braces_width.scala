// expect: 13:34: error: value b is not a member of A
// A line break inside the parentheses of an extension's header gives the braces around the header their width
// from outside them, as it does for every later line of those braces: the line after `val a =` is then indented
// against that width and begins the val's right side, a block that holds the next val too. scalac: "value b is
// not a member of object A".
object A { extension (x: Int)(using
    y: Int) def f = x + y
  val a =
  1
  val b = 2
}

@main def main(): Unit = println(A.b)
