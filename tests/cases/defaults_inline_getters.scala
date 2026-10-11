// An inline method's defaults are its getters' applications too (`Desugar.addDefaultGetters`):
// a later parameter's getter is an ordinary method that takes the earlier parameters strictly,
// so an inline parameter's default runs once as its argument even where the body never uses
// the parameter, and again at each use of it.
object InlineGetters:
  def mark(s: String): Int = { print(s); 1 }
  inline def f(inline a: Int = mark("A"))(b: Int = mark("B")): Unit = ()
  inline def g(inline a: Int = mark("A"))(b: Int = mark("B")): Int = a + a
  inline def h(a: Int = mark("A"))(b: Int = a + mark("B")): Int = a + b

  def main(args: Array[String]): Unit =
    f()(); println()
    println(g()())
    println(h()())
    f(mark("a"))(); println()
