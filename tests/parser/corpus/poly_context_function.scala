// A polymorphic function type over a context function, `[A <: P] => T[A] ?=> Out`: the literal
// takes its context parameter by `?=>`, an application passes it with `using` or resolves it from
// the givens in scope.
trait P
final class T[A](val name: String)
final class Box[A](val v: String)
final class Q extends P

def applyF[Out](f: [A <: P] => T[A] ?=> Out)(x: Int): Out =
  f[P](using T[P](s"t$x"))

def applyGiven[Out](f: [A <: P] => T[A] ?=> Out): Out =
  given T[Q] = T[Q]("given")
  f[Q]

def mk[A <: P](using t: T[A]): Box[A] = Box[A](t.name)

@main def main(): Unit =
  val r: Box[?] = applyF[Box[?]]([A <: P] => (t: T[A]) ?=> mk[A])(3)
  println(r.v)
  val s = applyF[String]([A <: P] => (_: T[A]) ?=> summon[T[A]].name)(4)
  println(s)
  val named: [A <: P] => T[A] ?=> String = [A <: P] => (t: T[A]) ?=> t.name + "!"
  println(applyGiven(named))
  println(named[Q](using T[Q]("direct")))
