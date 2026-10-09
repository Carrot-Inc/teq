package rea

// Transparent methods expanded in the upstream: two of one shape the whole build outlines into
// one function (with a third in a sibling block), expansions with bindings, one beside a local
// of its own name, and expansions whose type test reads the call's type argument (the
// outlining's replay).
object Fmt:
  transparent inline def show[T](inline label: String, x: Int, y: T): String =
    val pre = label + ": "
    val k = pre.length + x
    pre + x.toString + (if k > 10 then "!" else ".") + k.toString + " " + y.toString

  transparent inline def kind[T](x: Any): String =
    val a = x.isInstanceOf[T]
    val b = x.toString.length
    if a then "yes " + b.toString + " " + x.toString else "no " + b.toString + " " + x.toString

object Use:
  def a: String = Fmt.show[Int]("alpha", 1, 10)
  def b: String = Fmt.show[String]("beta", 2, "z")
  def c: String = Fmt.show[Int]("x", 1, 2) + Fmt.show[Int]("y", 2, 3)
  def d(n: Int): String =
    val pre = "outer"
    pre + Fmt.show[Int]("w", n, 4)
  def bound(n: Int): String = Fmt.show[Int]("b", n + 1, n * 2)
  def bound2(n: Int): String = Fmt.show[Int]("c", n + 2, n * 3)
  def kinds(v: Any): String = Fmt.kind[String](v) + Fmt.kind[Int](v) + Fmt.kind[String](v.toString + "!")
