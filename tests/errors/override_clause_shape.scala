// Two methods whose clauses pair up one by one, the one's extra clauses empty, are one member whose type differs, an
// incompatible type (E164), as `def f` against `def f()` is: dotty's `RefChecks` reports it of a pair whose types match
// loosely (`trueMatch`, `Type.matches` relaxed), where teq took `f(x: Int)()` for an overload of `f(x: Int)`.
// (`m(x: Int, y: Int)` against `m(x: Int)(y: Int)` is no override: tests/errors/override_rules.)
// expect: 7:25: error: error overriding method f in trait A of type (x: Int): Int: method f of type (x: Int)(): Int has incompatible type
trait A { def f(x: Int): Int = 1 }
trait B extends A { def f(x: Int)(): Int }
class C extends B { def f(x: Int)(): Int = 2 }
@main def run() = println((new C).f(1)())
