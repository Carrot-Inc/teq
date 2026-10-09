// jars: zio
// zio's `Chunk` from its jar: `sorted` picks an array sort by the `Ordering` and `ClassTag`
// instances it names, the builders are scala-library's `ArrayBuilder.ofX`, and a nested class
// reads the companion's `Tags` object through the trait that defines it.
//> using dep dev.zio::zio:2.1.26
import zio.Chunk

object Main:
  def main(args: Array[String]): Unit =
    println(Chunk(3, 1, 2).sorted)
    println(Chunk.fromArray(Array("b", "a")).sorted)
    println((Chunk(1.5, 0.5) ++ Chunk(2.25)).sorted)
    println(Chunk.fromIterable(List('c', 'a')).sorted)
    println(Chunk.single(4L).sorted)
    val b = Chunk.newBuilder[Int]
    b += 1
    b += 2
    b ++= List(3, 4)
    println(b.result())
    val bytes = Chunk.fromArray(Array[Byte](3, 1, 2))
    println(bytes.sorted.map(_ + 1))
    println(Chunk("x", "y").zipWithIndex.toList)
    println(Chunk(1, 2, 3).map(_ * 2).filter(_ > 2).sum)
