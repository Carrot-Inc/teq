// `given_index_dependent_parameter_same_clause` with the dependency across clauses (`(using t: Key)(using x: t.Out)`):
// `normalize` replaces each clause's term parameters in turn (ProtoTypes.scala:928-934), so the later clause's
// parameter type is a wildcard too (scalac 3.8.4: 13).
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
  given proxy(using t: Key)(using x: t.Out): x.type = x
val lib = new Lib
object Main:
  def main(args: Array[String]): Unit =
    import lib.given
    println(summon[A].n)
