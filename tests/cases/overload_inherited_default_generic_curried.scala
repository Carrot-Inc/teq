// A curried overload whose inherited default is a generic parent's: an override of A[X].m(a: T)(b: Int = 1) without
// the default beside an alternative with none, and the mirror over a trait TQ[Y]; m(Z)() applies the parent's default,
// the inheriting override declared second (the order scalac 3.8.4 accepts: overload_inherited_default_order). scalac
// prints 11 and 22. teq's JVM build calls the getter by the override's descriptor (B.m$default$2(X)), which no class
// declares, and dies at run: a row of master's own, which dies the same way without an overload (the dispatched
// getter's descriptor over a generic parent), listed pending on the JVM under it.
object Gen {
  trait X; trait Y; object Z extends X with Y
  class A[T] { def m(a: T)(b: Int = 1): Int = b + 0 }
  class B extends A[X] { def m(a: Y)(b: Int): Int = b + 20; override def m(a: X)(b: Int): Int = b + 10 }
  trait TQ[T] { def m(a: T)(b: Int = 2): Int = b + 30 }
  class C extends TQ[Y] { def m(a: X)(b: Int): Int = b + 10; override def m(a: Y)(b: Int): Int = b + 20 }
}
@main def run(): Unit =
  println(new Gen.B().m(Gen.Z)())
  println(new Gen.C().m(Gen.Z)())
