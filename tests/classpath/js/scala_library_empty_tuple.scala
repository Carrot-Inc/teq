// jars: scala-library zio zio-stacktracer zio-internal-macros izumi-reflect izumi-reflect-boopickle scala-collection-compat macrotask-executor scala-java-time
//> using platform js
//> using dep dev.zio::zio::2.1.26
// `EmptyTuple` with scala-library's jar on the class path is the std's: the jar's
// `Tuple$package.EmptyTuple` maps onto it, so a `*:` chain ending in it is the pair it spells
// (zio's `MemoMap.getOrElseMemoize` builds a layer's `(Patch, ZEnvironment)` as one).
import zio.*

object Main extends ZIOAppDefault:
  val tagged: ZLayer[Any, Nothing, String] = ZLayer.succeed("layer")
  def run =
    val t: Int *: String *: EmptyTuple = (1, "a")
    val pair: (Int, String) = t
    for
      s <- ZIO.service[String].provideLayer(tagged)
      _ <- Console.printLine(s"$s ${pair._2} ${t.size}")
      _ <- Console.printLine((t ++ (true *: EmptyTuple)).toList.mkString(","))
    yield ()
