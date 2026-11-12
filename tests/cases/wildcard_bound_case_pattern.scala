// No bounds are checked in a pattern: `Checking.checkAppliedType` skips them under `Mode.Pattern` (Checking.scala
// 125, "no bounds checking in patterns"), which `PostTyper` sets for a typed pattern `y: T` (its
// `Typed(Ident(nme.WILDCARD), _)` case, PostTyper.scala 682-689), so `case y: C[? >: String]` of `C[T <: Int]`,
// also nested (`List[C[? >: String]]`), is accepted by scalac 3.8.4 (the second with `@unchecked`). teq agrees,
// for a non-wildcard argument as well (`case y: C[String]`).
class C[T <: Int]
def f(x: Any): Int = x match
  case y: C[? >: String] => 1
  case y: List[C[? >: String]] @unchecked => 2
  case _ => 3
@main def main(): Unit = println(List(f(1), f(List(1)), f(new C[Int])).mkString(","))
