// expect: 21:71: error: Found:    (1 : Int)
// expect: 1 error found
// Adapted from scala3 tests/neg/i2887c.scala (Apache-2.0, see tests/scala3/README.md): an abstract higher-kinded
// member applied, through a dependent method's parameters, to arguments of other kinds. scalac 3.8.4 rejects the
// argument `1`: its expected type is the unreduced lambda application `([X[_,_], Y[_], Z] =>> X[Z, Y[Z]])[[X] =>> X,
// [X] =>> X]` (a parameter met with no argument, TypeApplications.scala 401-402). teq's node of that application
// (`Type::Applied`, dotty's `AppliedType(self, args)` over the lambda) keeps its constructor and arguments through the
// dependent approximation and the comparison, `Any` its `superType` and nothing but `Nothing` below it, and the
// argument is rejected as scalac rejects it; the error type had accepted it, and a wildcard, read as bounds once
// passed on, had become `Any`.
trait A { type S[X[_,_], Y[_],_]; type I[_] }
trait B { type S[X[_],Y[_]]; type I[_,_] }
trait C { type M <: B }
trait D { type M >: A }

object Test {
  def test(x: C & D): Unit = {
    def foo(a: A, b: B)(z: a.S[b.I,a.I,b.S[a.I,a.I]]) = z
    def bar(a: A, y: x.M) = foo(a,y)
    def baz(a: A) = bar(a, a)
    baz(new A { type S[X[_,_], Y[_], Z] = X[Z,Y[Z]]; type I[X] = X })(1) // error
  }
}
// expect: Required: ([X[_$10,_$11], Y[_$12], Z] =>> X[Z, Y[Z]])[[X²] =>> X², [X²] =>> X²]
