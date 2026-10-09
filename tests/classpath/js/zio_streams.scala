// jars: zio zio-stacktracer zio-internal-macros izumi-reflect izumi-reflect-boopickle scala-collection-compat macrotask-executor scala-java-time zio-streams
//> using dep dev.zio::zio:2.1.26
//> using dep dev.zio::zio-streams:2.1.26
// zio-streams 2.1.26 from its jar: `fromIterable`, `fromZIO`, `fromIterableZIO`, `mapZIO`,
// `filter`, `take`, `++`, `runCollect`, `runFold` and `runDrain` with a `Ref` counting what
// passed. A `ZIOAppDefault`, since the stream's fiber yields and Scala.js cannot block on
// `unsafe.run` ("Cannot block for result to be set in JavaScript").
package ziostreams

import zio.*
import zio.stream.*

object Main extends ZIOAppDefault:
  def run =
    val program =
      for
        seen <- Ref.make(0)
        a <- ZStream.fromIterable(1 to 6).filter(_ % 2 == 0).mapZIO(n => seen.update(_ + 1).as(n * 10)).runCollect
        b <- (ZStream.fromZIO(ZIO.succeed("head")) ++ ZStream("x", "y")).map(_.toUpperCase).runCollect
        c <- ZStream.fromIterableZIO(ZIO.succeed(List(5, 4, 3))).take(2).runFold(0)(_ + _)
        _ <- ZStream.range(0, 4).tap(_ => seen.update(_ + 1)).runDrain
        n <- seen.get
      yield s"$a $b $c $n"
    program.flatMap(Console.printLine(_))
