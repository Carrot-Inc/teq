// expect: 6:26: error: type mismatch: found Int => Int, required (=> Int) => Int
// A by-name parameter of a function type takes a thunk, so a function of the value is no such
// function (scalac: E007 "Found: Int => Int, Required: (=> Int) => Int").
def twice(t: => Unit) = { t; t }
def pipe(t: => Unit, f: (=> Unit) => Unit) = f(t)
val b: (=> Int) => Int = (a: Int) => a
def ok(x: => Int, ys: Int*): Int = x + ys.sum
class C(x: => Int, ys: Int*)
@main def run(): Unit = println(ok(1, 2, 3))
