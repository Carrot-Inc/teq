// jars: zio zio-stacktracer zio-internal-macros izumi-reflect izumi-reflect-boopickle scala-collection-compat macrotask-executor scala-java-time
//> using dep dev.zio::zio:2.1.26
// A class of a jar that overrides a method whose `ClassTag` the std erases on JavaScript
// (zio's `Chunk.toArray[B: ClassTag]` over the std's `IterableOps.toArray`): a call through the
// std, which passes no tag, and one of the jar's own, which passes it, both give the array
// (zio-test renders a failed assertion through the first).
import zio.Chunk

@main def run(): Unit =
  val c = Chunk(3, 1, 2)
  println(c.sorted)
  println(c.toArray.toList)
  val s: Seq[Int] = c
  println(s.sorted)
  println(s.toArray.mkString(","))
  println(c.map(_ * 2).reverse)
  val it: Iterable[String] = Chunk("b", "a")
  println(it.toList.sorted)
