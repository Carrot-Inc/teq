// jars: scala-library
// The array test on the JVM where scalac tells the test uncheckable and still makes it of the element's erased
// class: a value class with `Null` and a universal trait with `Null` are `Object[]`s (no warning), a `*:` chain
// spelled with `EmptyTuple` is a `Product[]`, a refinement its parent's array, a structural type and a polymorphic
// function type the arrays of `Object` and of the function class. The warnings are scalac's, at its lines.
object ValueClassNull:
  class V(val x: Int) extends AnyVal
  def run(): Unit =
    val strs: Any = Array("a")
    val vs: Any = Array(new V(1))
    println("1 " + strs.isInstanceOf[Array[V | Null]] + " " + vs.isInstanceOf[Array[V | Null]])
    println("2 " + strs.isInstanceOf[Array[Null | V]])
    println("3 " + strs.isInstanceOf[Array[V | Null] | Array[Int]])
object Chain:
  def run(): Unit =
    val ones: Any = Array(Tuple1(1))
    val pairs: Any = Array((1, "a"))
    val strs: Any = Array("a")
    println("1 " + ones.isInstanceOf[Array[Int *: String *: EmptyTuple]] + " " + strs.isInstanceOf[Array[Int *: String *: EmptyTuple]])
    println("2 " + pairs.isInstanceOf[Array[Int *: EmptyTuple]] + " " + strs.isInstanceOf[Array[Int *: EmptyTuple]])
object Refined:
  class Box(val n: Int)
  def run(): Unit =
    val strs: Any = Array("a")
    val ints: Any = Array(1)
    val boxes: Any = Array(new Box(1))
    println("1 " + strs.isInstanceOf[Array[Box { def n: Int }]] + " " + ints.isInstanceOf[Array[Box { def n: Int }]] + " " + boxes.isInstanceOf[Array[Box { def n: Int }]])
    println("2 " + (strs match { case _: Array[Box { def n: Int }] => true; case _ => false }))
    println("3 " + ints.isInstanceOf[Array[{ def n: Int }]])
    println("4 " + strs.isInstanceOf[Array[[T] => T => T]])
object RefinedLayers:
  class Box(val n: Int)
  def run(): Unit =
    val s1: Any = Array("a")
    val s2: Any = Array(Array("a"))
    val i2: Any = Array(Array(1))
    val b2: Any = Array(Array(new Box(1)))
    println("" + s1.isInstanceOf[Array[Array[Box { def n: Int }]]] + " " + s2.isInstanceOf[Array[Array[Box { def n: Int }]]] + " " + i2.isInstanceOf[Array[Array[Box { def n: Int }]]] + " " + b2.isInstanceOf[Array[Array[Box { def n: Int }]]])
@main def run(): Unit =
  ValueClassNull.run()
  Chain.run()
  Refined.run()
  RefinedLayers.run()
