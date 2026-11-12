// Adapted from scala3 tests/run/i16879.scala (Apache-2.0, see tests/scala3/README.md).
// Pending: D12, the printing of a zero-parameter case class's companion; R8, a failing assert.
trait Companion:
  final override def toString: String = "Companion"

case class Example(value: Int)
object Example extends Companion

case class C()
object C:
  override def toString = "CC"

case class D()

@main def Test =
  assert(Example.toString == "Companion")
  assert(C.toString == "CC")
  assert(D.toString == "D")
