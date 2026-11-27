package mlta

// Two traits override a generic member, one keeping the type parameter and one fixing it: the fixing one writes a
// bridge of the root's erasure, the keeping one none.
trait A[T] { def m(a: T): String = "A" + a }
trait Keep[T] extends A[T] { override def m(a: T): String = "K" + a }
trait Fix extends A[Int] { override def m(a: Int): String = "F" + a }
