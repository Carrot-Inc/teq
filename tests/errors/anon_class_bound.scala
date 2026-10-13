// An anonymous class's instance is typed as dotty's `TypeOps.classBound` gives it: its parents refined by
// a declaration only where it needs to be (`addRefinement`): a member a parent has, where the declaration
// is strictly narrower (`type T = Int` of an abstract `T`, `def get: String` of `def get: Any`), and any
// public member where a parent derives from `Selectable` (`pos/i9667`). A type or value no parent has is
// no member of the instance otherwise.
trait A:
  type T
  type M
  def get: Any
val a = new A { type T = Int; type U = String; type M = Int; def get: String = "s" }
val t: a.T = 1
val u: a.U = "s"
val g: String = a.get
class P
val p = new P { type U = Int; val v = 1 }
val pu: p.U = 1
val pv = p.v
val bar = new reflect.Selectable {
  class C extends A { def get = 1 }
  type B
  type D = C
  val x: C = ???
  val xx: x.M = ???
  val y: B = ???
}
val bar1: reflect.Selectable { type B; type D <: A; val x: A; val xx: Any; val y: this.B } = bar
// expect: anon_class_bound.scala:12:8: error: type U is not a member of
// expect: anon_class_bound.scala:16:9: error: type U is not a member of
// expect: anon_class_bound.scala:17:10: error: value v is not a member of P
// expect: 3 errors found
