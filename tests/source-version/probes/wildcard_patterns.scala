type M[X] = X match { case List[_] => Int; case _ => String }
trait F[G[_]]
object P { def f(x: AnyRef) = x match { case _: List[_] => 1; case _: Map[_, _] => 2; case _ => 0 }; val x: List[_] = Nil; val g = (y: AnyRef) => y match { case _: Option[_] => 1 }; val h: Map[_ <: AnyRef, _] = Map() }
