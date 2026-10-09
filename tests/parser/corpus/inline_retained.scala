// An inline method that overrides or implements a member that is not inline keeps a method of
// its own, so that a call through the supertype dispatches to it; a call on the subclass
// expands. Adapted from scala3 tests/run/inline-override.scala (Apache-2.0, see
// tests/scala3/README.md).
abstract class Formatter:
  def plain(x: Int) = s"dynamic $x"
  def header(x: Int): String
  def footer(x: Int): String
  inline def tag(x: Int): String

class Bold extends Formatter:
  inline override def plain(x: Int) = wrap(x)
  inline def wrap(x: Int) = s"inline $x"
  inline def header(x: Int) = wrap(x)
  inline def footer(x: Int) = wrap(x)
  inline def tag(x: Int) = wrap(x)

object Main:
  def main(args: Array[String]): Unit =
    val b = Bold()
    println(b.plain(22))
    println(b.header(22))
    println(b.footer(22))
    println(b.tag(22))
    val a: Formatter = b
    println(a.plain(22))
    println(a.header(22))
    println(a.footer(22))
