// An unapplied method whose clause has defaults stays a function type under a selection: refused.
// expect: 5:33: error: value && is not a member of Int => C
class C { def &&(n: Int): Int = n }
object O { def f(n: Int = 1): C = new C }
@main def run(): Unit = println(O.f && 6)
