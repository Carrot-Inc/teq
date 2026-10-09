// jars: zio zio-stacktracer zio-internal-macros izumi-reflect izumi-reflect-boopickle scala-collection-compat macrotask-executor scala-java-time
//> using dep dev.zio::zio:2.1.26
// `ZLayer` and `ZEnvironment` over services looked up by `Tag`, which izumi-reflect's macro
// derives: `ZLayer.succeed`, `ZLayer.fromFunction`, a layer built from an effect, `ZIO.service`,
// `serviceWith`, `provide` and `provideEnvironment`, and the tags themselves compared. A
// `ZIOAppDefault`, since building a layer suspends and Scala.js cannot block on `unsafe.run`.
package ziolayers

import zio.*

trait Greeter:
  def greet(name: String): String

trait Counter:
  def next: UIO[Int]

final case class Settings(prefix: String)

object Main extends ZIOAppDefault:
  val settings: ULayer[Settings] = ZLayer.succeed(Settings("hi"))
  val greeter: URLayer[Settings, Greeter] = ZLayer.fromFunction((s: Settings) => new Greeter:
    def greet(name: String) = s"${s.prefix} $name")
  val counter: ULayer[Counter] = ZLayer(Ref.make(0).map(r => new Counter:
    def next = r.updateAndGet(_ + 1)))

  val program: ZIO[Greeter & Counter, Nothing, String] =
    for
      g <- ZIO.service[Greeter]
      _ <- ZIO.serviceWithZIO[Counter](_.next)
      n <- ZIO.serviceWithZIO[Counter](_.next)
      pre <- ZIO.serviceWith[Greeter](_.greet("there"))
    yield s"${g.greet("you")} / $pre / $n"

  def tags(): Unit =
    println(Tag[Int].tag.shortName)
    println(Tag[List[String]].tag.repr)
    println(Tag[Greeter & Counter].tag.repr)
    println(Tag[Either[String, Option[Int]]].tag.shortName)
    println(Tag[Settings].tag <:< Tag[Product].tag)
    println(Tag[List[Int]].tag <:< Tag[Seq[Any]].tag)
    println(Tag[Greeter].tag =:= Tag[Counter].tag)

  def run =
    val env = ZEnvironment(Settings("env"), 3)
    for
      _ <- ZIO.succeed(tags())
      _ <- Console.printLine(env.get[Settings].prefix + " " + env.get[Int])
      out <- program.provide(settings, greeter, counter)
      _ <- Console.printLine(out)
      direct <- ZIO.serviceWith[Settings](_.prefix).provideEnvironment(env)
      _ <- Console.printLine(direct)
    yield ()
