// jars: zio zio-stacktracer zio-internal-macros izumi-reflect izumi-reflect-boopickle scala-collection-compat macrotask-executor scala-java-time
//> using dep dev.zio::zio:2.1.26
// A class of a jar that takes a method of the std beside one of a trait of its own under one
// name (zio's `Chunk.BooleanArray`: `IterableOps.++` and `ChunkIterator.++`) names the jar's
// apart: the std's classes keep calling theirs by the name they define it under.
import zio.Chunk

@main def run(): Unit =
  println(Chunk(1) ++ Chunk(2))
  println(Chunk(true, false) ++ Chunk(true))
  val lists: Seq[Iterable[Int]] = Seq(List(1) ++ List(2), Nil ++ List(3), Vector(4) ++ List(5))
  println(lists.map(_.toList))
  val appended: Iterable[Int] = (Nil: Iterable[Int]) ++ List(6)
  println(appended.toList)
