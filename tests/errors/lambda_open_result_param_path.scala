// A variable of a lambda's body bounded through the lambda's parameter is settled inside the lambda,
// where dotty's instantiation of the closure's result avoids the deeper parameter (`fixLevels`, its
// `AvoidMap` reading the open variable's instance): `call(x => Item("t", x.a), s)` is an `Item[?]`
// as scalac infers it, never an `Item[x.A]` naming a parameter out of scope.
trait S { type A; val a: A }
final case class Item[T](title: String, value: T)
def call[R](f: S => R, s: S): R = f(s)
def run(s: S) =
  val r = call(x => Item("t", x.a), s)
  val z: Int = r
// expect: 10:16: error: type mismatch: found Item[_], required Int
// absent: x.A
