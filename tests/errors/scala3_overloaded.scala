// Adapted from scala3 tests/neg/overloaded.scala (Apache-2.0, see tests/scala3/README.md): the
// statements moved into a method.
// expect: 15:5: error: Ambiguous overload. The overloaded alternatives of method foo in object Test with types
// expect:  (f: Int => Int): String
// expect:  (f: Char => Char): Unit
// expect: both match arguments (<?>)
// expect: 1 error found
object Test {
  def mapX(f: Char => Char): String = ???
  def mapX[U](f: U => U): U = ???

  def foo(f: Char => Char): Unit = ???
  def foo(f: Int => Int): String = ???
  def run(): Unit =
    foo(x => x) // error: ambiguous

  def bar(f: (Char, Char) => Unit): Unit = ???
  def bar(f: Char => Unit) = ???
  def ok(): Unit =
    mapX(x => x) //OK
    bar((x, y) => ())
    bar (x => ())
}
