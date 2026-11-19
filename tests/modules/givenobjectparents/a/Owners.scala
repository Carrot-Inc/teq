package gop

// Given objects of several parents in generic classes, read from the module's products by its user: each is found
// as each of its parents, the class's type parameter seen from the instance or the object it is read on (dotty's
// `c.pair.type`, whose base types are seen from the prefix).
class P[T] { def p(t: T): String = "p" + t.toString }
trait R { def r: Int = 2 }
class C[T]:
  given pair: P[T]() with R with {}
object O extends C[Int]
class Q { def x: Int = 1 }
trait S[T] { def s(t: T): String = "s" + t.toString }
class D[T]:
  given both: Q() with S[T] with {}
object E extends D[Int]
