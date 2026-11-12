// A given whose result depends on its `using` clause through another parameter's type (`x: t.Out`, `x.type`): dotty's
// `candidateKind` normalizes it through `resultTypeApprox(mt, wildcardOnly = true)`, the method's term parameters
// replaced by wildcards (ProtoTypes.scala:899-909), so its class is unknown and the search tries it: `proxy` answers
// `summon[A]` with `B`'s `b` (scalac 3.8.4: 13).
trait Base
trait A extends Base:
  def n: Int
class B extends A:
  def n = 13
object B:
  given b: B = new B
trait Key:
  type Out <: Base
object Key:
  object value extends Key:
    type Out = B
  given key: value.type = value
class Lib:
  given proxy(using t: Key, x: t.Out): x.type = x
val lib = new Lib
object Main:
  def main(args: Array[String]): Unit =
    import lib.given
    println(summon[A].n)
