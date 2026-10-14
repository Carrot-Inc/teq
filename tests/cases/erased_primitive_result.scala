// A call whose declared result is a type parameter erases to a reference, so where it is a primitive it may be
// `null` (a missing key of a `java.util.HashMap[String, Int]`): consumed as the primitive (a val or a var, a
// parameter of the primitive's type, an operator's operand, a match's selector, a varargs element, a closure's
// or a method's result, a branch of a conditional or a block's result) it is unboxed to the zero, as dotty's
// `Erasure.Boxing.adaptToType` unboxes it (`BoxesRunTime.unboxToInt`); passed where `Any` is expected
// (`println`, an interpolation, a concatenation, a generic parameter such as `Option.apply`'s) it stays `null`. An
// inline method's call is its expansion, which dotty erases after `Inlining`: a transparent one's root is unboxed
// as it is consumed, a plain one's always, as its body is ascribed its declared result (`PrepareInlineable.wrapRHS`).
object Main:
  inline def getInline[V](m: java.util.HashMap[String, V], k: String): V = m.get(k)
  transparent inline def head[A](xs: List[A]): A = xs.head
  transparent inline def getTransparent[V](m: java.util.HashMap[String, V], k: String): V = m.get(k)
  def f(i: Int): Int = i
  def g(a: Any): Any = a
  def id[A](x: A): A = x
  def first[T](xs: Array[T]): T = xs(0)
  def main(args: Array[String]): Unit =
    val m = new java.util.HashMap[String, Int]()
    val k = "r"
    val i: Int = m.get(k); println(s"val typed $i")
    val j = m.get(k); println(s"val inferred $j")
    var v: Int = 5; v = m.get(k); println(s"var assigned $v")
    println(s"arg Int ${f(m.get(k))}")
    println(s"arg Any ${g(m.get(k))}")
    println(s"interp ${m.get(k)}")
    println(m.get(k))
    println("concat " + m.get(k))
    println(s"plus ${m.get(k) + 1}")
    println(s"eq0 ${m.get(k) == 0}")
    println(s"if ${if (m.get(k) == 0) "zero" else "other"}")
    println(s"match ${m.get(k) match { case 0 => "zero"; case _ => "other" }}")
    def r: Int = m.get(k); println(s"def typed $r")
    def s = m.get(k); println(s"def inferred $s")
    println(s"lambda ${List(k).map(x => m.get(x))}")
    println(s"lambda typed ${List(k).map[Int](x => m.get(x))}")
    println(s"tuple ${(m.get(k), 1)._1 + 1}")
    println(s"option ${Option(m.get(k)).getOrElse(-1)} ${Some(m.get(k)).get + 1}")
    println(s"generic array ${first(Array[Any](null)).asInstanceOf[Int] + 1}")
    println(s"id ${id(null).asInstanceOf[Int]}")
    println(s"max ${m.get(k).max(3)}")
    println(s"list ${List(m.get(k))}")
    val arr = Array(m.get(k)); println(s"array ${arr(0)}")
    println(s"getOrDefault ${m.getOrDefault(k, 7) + 1}")
    val inferredIf = if args.length == 0 then m.get(k) else 0
    println(s"if inferred $inferredIf")
    val inlined: Int = getInline(m, k); println(s"inline expansion ${inlined + 1}")
    val inlinedInferred = getInline(m, k); println(s"inline inferred ${inlinedInferred + 1}")
    println(s"transparent ${head(List(3, 4)) * 2}")
    println(getInline(m, k))
    println(getInline(new java.util.HashMap[String, Int](), k))
    val transparentTyped: Int = getTransparent(m, k); println(s"transparent typed ${transparentTyped + 1}")
    println(getTransparent(m, k))
