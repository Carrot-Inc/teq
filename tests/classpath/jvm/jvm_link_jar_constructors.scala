// jars: scala-library
// std: scala-library
// Jar constructors are called with their class files' descriptors: `ArraySeq.ofRef` takes an
// `Object[]` where teq erases `Array[T <: AnyRef]` to `Object`.
import scala.collection.immutable.ArraySeq
import scala.collection.mutable
@main def run(): Unit =
  val a = new ArraySeq.ofRef[String](Array("a", "b"))
  println(a)
  val w = new mutable.ArraySeq.ofInt(Array(1, 2))
  println(w.sum)
  val t = new scala.util.matching.Regex("a(b)", "g")
  println(t.findFirstMatchIn("xab").map(_.group("g")))
