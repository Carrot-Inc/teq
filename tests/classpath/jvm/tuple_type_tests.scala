// jars: scala-library
// std: scala-library
// A tuple is a `Product` of a tuple class to the JVM: `isInstanceOf[Tuple]` and `case t: Tuple` go through
// `Tuples.isInstanceOfTuple`, as scalac tests them (`NonEmptyTuple` and `*:` through its sibling).
object Api:
  def a[T <: Tuple](t: T): T = t
  def b(t: Int *: String *: EmptyTuple): Int = t.head
  def c(t: NonEmptyTuple): Int = t.size
object Main:
  def main(args: Array[String]): Unit =
    val x: Any = (1, 2)
    val e: Any = EmptyTuple
    val s: Any = "s"
    println(x.isInstanceOf[Tuple])
    println(x.isInstanceOf[NonEmptyTuple])
    println(e.isInstanceOf[Tuple])
    println(e.isInstanceOf[EmptyTuple])
    println(s.isInstanceOf[Tuple])
    println(x match { case t: Tuple => s"tuple ${t.size}"; case _ => "other" })
    println(Api.a((1, "x")))
    println(Api.b((1, "x")))
    println(Api.c((1, 2, 3)))
