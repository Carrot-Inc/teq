// scala-library's `AnyRefMap[K <: AnyRef, V]` has inner classes of their own type parameters
// (`AnyRefMapIterator[A]`): completing one keeps the bound of the outer `K`, which the bodies of
// `update` and `hashOf` rely on.
import scala.collection.mutable.AnyRefMap

@main def run(): Unit =
  val m = AnyRefMap.empty[String, Int]
  m.update("a", 1)
  m("b") = 2
  println(m.toList.sorted)
  println(m.iterator.map(_._2).sum)
