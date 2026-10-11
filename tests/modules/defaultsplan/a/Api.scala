package dplan

// Calls whose omitted arguments are their default getters' applications, in the plan scalac's
// typer makes (`Applications.TypedApply`): the products write it and the reader keeps it, so
// that a downstream module runs it, and expands the inline bodies over it, as the whole build does.
object Api:
  def mark(s: String): Int = { print(s); 1 }
  def f(a: Int = mark("A"))(using u: Int)(b: Int = mark("B")): Unit = ()
  def usingClause(): Unit = { f()(using mark("U"))(); println() }
  def g(a: => Int)(b: Int = a): Int = a + b
  def byName(): Unit = println(g({ val x = mark("T"); x })())
  def h(xs: Int*)(b: Int = { print("B"); xs.sum }): Int = b + xs.sum
  def repeated(): Unit = println(h(mark("A"))())
  def kinds(xs: Int*)(b: Int = xs.size): String = (if xs.isInstanceOf[List[?]] then "list" else "array") + b
  inline def inlineRepeated(): Unit = println(kinds(mark("A"), mark("B"))())
  def k(a: Int = mark("A"))(b: Int): Unit = ()
  inline def inlineClause(): Unit = { k()(mark("B")); println() }
  inline def inlineUsing(): Unit = { f()(using mark("U"))(); println() }
  extension (r: Int) def +:(a: Int)(b: Int = mark("B")): Int = a + r + b
  def rightAssoc(): Unit = println((mark("A") +: mark("R"))())
  extension (r: String) def ++:(a: String)(b: String = a + "/" + r): String = b
  inline def inlineRightAssoc(): Unit = println((s("A") ++: s("R"))())
  def s(x: String): String = { print(x); x }
  class C(a: Int = mark("A"), b: Int)(c: Int, d: Int)
  def ctor(): Unit = { new C(b = 0)(d = mark("D"), c = mark("C")); println() }
