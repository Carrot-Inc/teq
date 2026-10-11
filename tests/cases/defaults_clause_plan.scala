// An omitted argument is its default getter's application, evaluated where scalac's plan puts
// it (`Applications.TypedApply`): a later clause applied with defaults first lifts the receiver
// and the earlier clauses' arguments (`liftFun`), so an earlier clause's default runs before a
// later clause's explicit argument, a using one included; a default of a using clause with no
// given runs before the next clause; a throwing default stops the call before a later argument.
object Plan:
  def mark(s: String): Int = { print(s); 1 }
  def f(a: Int = mark("A"))(using u: Int)(b: Int = mark("B")): Unit = ()
  class C { def g(a: Int = mark("A"))(b: Int): Unit = () }
  def receiver(): C = { print("R"); new C }
  def h(using u: Int = mark("U"))(b: Int): Unit = ()
  def t(a: Int = throw new Exception("A"))(b: Int): Unit = ()
  def k(a: Int = mark("A"))(b: Int)(c: Int = mark("C")): Int = a + b + c

  def main(args: Array[String]): Unit =
    f()(using mark("U"))(); println()
    f(a = mark("a"))(using mark("U"))(b = mark("b")); println()
    receiver().g()(mark("B")); println()
    h(mark("B")); println()
    try t()({ print("B"); 1 }) catch { case _: Exception => println("caught") }
    println(k()(mark("B"))())
