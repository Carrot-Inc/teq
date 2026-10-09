// The calls come first among the files, as in inline_demanded_this: `twice` and `label` expand
// at the call, the expansion asks for the result types of `both` and `shown`, which are
// inferred, and their bodies are typed on that demand. The class's type parameter in them is
// the class's, not the type argument of the receiver the asking call has: read as the
// argument, `both` would be a `List[Int]` for every `Box`, and the anonymous class of `shown`
// would be named after the call.
package demandedtargs

object Uses:
  def count(): Int =
    val b = new Box[Int](3)
    b.twice

  def first(): String =
    val b = new Box[String]("x")
    b.both.head

  def named(): String =
    val b = new Box[String]("y")
    b.label

  def numbered(): String =
    val b = new Box[Int](7)
    b.label

@main def run(): Unit =
  println(Uses.count())
  println(Uses.first())
  println(Uses.named())
  println(Uses.numbered())
