// A case class's name applied, its `apply` written, named, taken as a function or as a value is the companion's
// synthesized `apply`: the primary constructor's parameters and access and nothing else (dotty's
// `NamerOps.needsConstructorProxies` gives a case class no constructor proxy; `Desugar.classDef` its `apply`). A
// secondary constructor is reached by `new`, a parent call or an anonymous class
// (tests/cases/case_apply_primary_alone.scala).
// expect: 32:11: error: too many arguments: expected 1
// expect: 33:11: error: too many arguments: expected 1
// expect: 34:13: error: type mismatch: found "abc", required Int
// expect: 35:11: error: there is no parameter named a
// expect: 35:11: error: there is no parameter named b
// expect: 35:11: error: missing argument for parameter x
// expect: 36:28: error: too many arguments: expected 1
// expect: 37:11: error: missing argument for parameter x
// expect: 38:11: error: missing argument for parameter x
// expect: 39:11: error: the constructor of S is private to S
// expect: 39:11: error: too many arguments: expected 1
// expect: 40:21: error: the constructor of S is private to S
// expect: 41:23: error: the constructor of S is private to S
case class V(x: Int):
  def this(a: Int, b: Int) = this(a + b)
  def this(s: String) = this(s.length)
case class S private (x: Int):
  def this(a: Int, b: Int) = this(a + b)
object O:
  case class K(x: Int):
    def this() = this(7)
trait Outer:
  case class In(x: Int):
    def this() = this(7)
object D extends Outer
@main def main(): Unit =
  println(V(1, 2))
  println(V.apply(1, 2))
  println(V("abc"))
  println(V(a = 1, b = 2))
  val f: (Int, Int) => V = V(_, _)
  println(O.K())
  println(D.In())
  println(S(1, 2))
  val g: Int => S = S.apply
  println(List(1).map(S.apply))
