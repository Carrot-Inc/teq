package kbn

class A(val v: Int)
class B(val v: Int)
class C(val v: Int)

// A short using list with named arguments around the searched one, in a class's parent and an enum case the library
// keeps: over the products the written ones stay in their places and the searched one is the given chosen there.
class Base(x: Int)(using a: A, b: B, c: C):
  def show: String = s"$x ${a.v} ${b.v} ${c.v}"
given B = B(200)
enum E(x: Int)(using a: A, b: B, c: C):
  def show: String = s"$x ${a.v} ${b.v} ${c.v}"
  case One extends E(1)(using a = A(1), c = C(3))

object Lib:
  given B = B(200)
  class Sub extends Base(1)(using a = A(1), c = C(3))
  def parent: String = (new Sub).show
  def enumCase: String = E.One.show
