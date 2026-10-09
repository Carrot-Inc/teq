// expect: covariant type A occurs in contravariant position in type A of parameter a
// expect: contravariant type M occurs in covariant position in type List[M] of value v
// expect: covariant type A occurs in invariant position in type A of variable x
// expect: contravariant type A occurs in covariant position in type A of method get
// expect: 4 errors found
class Foo[+A]:
  def foo(a: A): Unit = {}

class Bar[-M](val v: List[M])

class Cell[+A](var x: A)

trait Sink[-A]:
  def get: A

@main def run(): Unit = println(1)
