// Two `using` lists on an overloaded method whose first using clauses differ: the second list does not choose between
// them where the first cannot tell its own clauses apart (scalac takes the most specific by the first list, then
// refuses the second, E007); teq keeps its own refusal.
// expect: 11:11: error: Ambiguous overload.
// expect: 15:11: error: Ambiguous overload.
class A
class B
class C
object Lead {
  def u(using a: A)(using b: B): Int = 1; def u(using a: Any)(using c: C): Int = 2
  val r = u(using A())(using C())
}
object Follow {
  def f(x: Int)(using a: A)(using b: B): Int = 1; def f(x: Int)(using a: Any)(using c: C): Int = 2
  val r = f(1)(using A())(using C())
}
