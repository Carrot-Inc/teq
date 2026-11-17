// jars: scala-library
// A `*:` chain spelled with `EmptyTuple` erases to `Product` for scalac wherever the type names it: behind an
// alias and under a member of a union too. An array of one is then a `Product[]` to its type test, which an array
// of any tuple class passes, and a cast to it takes one. (In an inline body read from a pickle:
// tests/modules/arraychaininline, by the cast, which no warning accompanies.)
type One = Int *: EmptyTuple
type Pair = Int *: String *: EmptyTuple
def alias(x: Any): Boolean = x.isInstanceOf[Array[One] @unchecked]
def alias2(x: Any): Boolean = x.isInstanceOf[Array[Pair] @unchecked]
def member(x: Any): Boolean = x.isInstanceOf[(Array[Int *: EmptyTuple] | Array[Int]) @unchecked]
def pattern(x: Any): Boolean = x match { case _: (Array[Int *: EmptyTuple] @unchecked) => true; case _ => false }
def castAlias(x: Any): String = try x.asInstanceOf[Array[One]].length.toString catch { case e: ClassCastException => "CCE" }
def castUnchecked(x: Any): String = try x.asInstanceOf[Array[Int *: EmptyTuple] @unchecked].length.toString catch { case e: ClassCastException => "CCE" }
@main def run(): Unit =
  val ones: Any = Array(Tuple1(1)); val pairs: Any = Array((1, "a")); val strs: Any = Array("s")
  def row(n: String)(f: Any => Any) = println(n + " " + List(ones, pairs, strs).map(f).mkString(" "))
  row("alias")(alias)
  row("alias2")(alias2)
  row("member")(member)
  row("pattern")(pattern)
  row("cast-alias")(castAlias)
  row("cast-unchecked")(castUnchecked)
