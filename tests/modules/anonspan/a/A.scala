package asa

// An anonymous class stands where its `new` does, to the end of its template (scalac's
// `TYPEDEF $anon`), not at the definition whose body makes it.
trait T:
  def n: Int

abstract class Base(val k: Int):
  def m: Int

object A:
  def f: T = new T { def n = 7 }
  val g: T =
    val inner = 2
    new T:
      def n = inner * 4
  def h(x: Int): Base = new Base(x) with T { def m = k + 1; def n = k }
