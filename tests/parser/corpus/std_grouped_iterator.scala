// `Iterator.grouped` and `sliding` give a `GroupedIterator`, with `withPartial` and
// `withPadding` for the last window, and `Vector.newBuilder` is a `VectorBuilder`, whose
// `+=` and `addOne` return the builder itself (`this.type`), as scala-library's do.
import scala.collection.immutable.VectorBuilder
import scala.collection.mutable.{ArrayBuffer, ListBuffer}
object Main:
  def loop(builder: VectorBuilder[Int], n: Int): Vector[Int] = if n == 0 then builder.result() else loop(builder += n, n - 1)
  def main(args: Array[String]): Unit =
    println(List(1, 2, 3, 4, 5).iterator.sliding(2).toList)
    println(List(1, 2, 3, 4, 5).iterator.sliding(2, 3).toList)
    println(List(1, 2, 3, 4, 5).iterator.sliding(3).withPartial(false).map(_.sum).toList)
    println(List(1, 2, 3, 4, 5).iterator.grouped(2).toList)
    println(List(1, 2, 3, 4, 5).iterator.grouped(2).withPartial(false).toList)
    println(List(1, 2, 3, 4, 5).iterator.grouped(2).withPadding(0).toList)
    println(List(1, 2).iterator.sliding(3).toList)
    println(List(1, 2).iterator.sliding(3).withPartial(false).toList)
    println(List.empty[Int].iterator.sliding(2).toList)
    println(loop(new VectorBuilder[Int](), 3))
    val vb = Vector.newBuilder[String]
    vb += "a"
    vb.addOne("b").addAll(List("c", "d"))
    println(s"${vb.result()} ${vb.knownSize}")
    val ab = ArrayBuffer(1)
    println((ab += 2).addOne(3).addAll(List(4)).toList)
    val lb = ListBuffer(1)
    println((lb += 2).addOne(3).result())
