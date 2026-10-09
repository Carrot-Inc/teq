// An anonymous class in a nested inline expansion reads a local of the expansion around it.
package inlinenestedanoncapture

trait Get:
  def get: Int

inline def inner(inline y: Int): Get = new Get:
  def get: Int = y + 1

inline def outer(x: Int): Int =
  val base = x * 10
  val g = inner(base + x)
  g.get + base

inline def twice(x: Int): Int =
  val first = outer(x)
  inner(first).get

@main def main(): Unit =
  println(outer(2))
  println(twice(3))
