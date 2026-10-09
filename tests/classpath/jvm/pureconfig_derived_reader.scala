// jars: scala-library pureconfig-core-jvm typesafe-config
// std: scala-library
//> using dep com.github.pureconfig::pureconfig-core:0.17.10
// A derived `ConfigReader` in link mode: the reader's `from` reads the trait's private
// `fieldMapping` through the accessor scalac names `T$$inline$fieldMapping` in the class file.
import pureconfig.*

final case class Server(host: String, port: Int) derives ConfigReader
final case class Conf(name: String, server: Server, tags: List[String]) derives ConfigReader

object Main:
  def main(args: Array[String]): Unit =
    val source = ConfigSource.string("""name = app, server { host = "h", port = 8080 }, tags = [a, b]""")
    println(source.load[Conf])
    println(ConfigSource.string("""name = app""").load[Conf].isLeft)
