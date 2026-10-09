// jars: scala-library zio-jvm zio-stacktracer-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm scala-collection-compat-jvm
// std: scala-library
//> using dep dev.zio::zio:2.1.26
// An object mixing in a jar trait with concrete vals in link mode: the class holds a field, its
// getter and the `T$_setter_$x_$eq` setter for each (`ZIOApp`'s `shuttingDown`,
// `ZIOAppDefault`'s `bootstrap` and `environmentTag`), and the trait's `$init$` stores through
// the setter when the object is made; a val the class overrides keeps its own, which the trait's
// setter stores into first and the class's initialiser then sets (`trait_setter_override`).
import zio.*

final class Overriding extends ZIOAppDefault:
  override val bootstrap: ZLayer[ZIOAppArgs, Any, Any] = ZLayer.succeed(1)
  def run = Console.printLine("overriding " + bootstrap.getClass.getSimpleName)

object App extends ZIOAppDefault:
  def run =
    Console.printLine("running") *> ZIO.succeed(bootstrap.getClass.getSimpleName).flatMap(Console.printLine(_)) *> new Overriding().run
