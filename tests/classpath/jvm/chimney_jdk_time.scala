// jars: scala-library chimney-jvm chimney-macro-commons-jvm scala-collection-compat-jvm
// std: scala-library
//> using dep io.scalaland::chimney:1.11.0
// chimney 1.11.0's `into[T].transform` in link mode on the JVM, where its macros run in teq's
// interpreter over scala-library's own collections: they time the derivation with the JDK's
// `Instant.now()`, which the interpreter answers natively (the output calls the JDK's), collect
// the source's fields in a scala-library `ListSet`, and print through scala-library's regular
// expressions, which run over the std's `java.util.regex`.
import io.scalaland.chimney.dsl.*
import java.time.{Duration, Instant}

final case class Event(at: Instant, name: String, count: Int)
final case class Stored(at: Instant, name: String, count: Int)
final case class Narrow(name: String)

object Main:
  def main(args: Array[String]): Unit =
    val e = Event(Instant.ofEpochSecond(1709214330L, 500000000L), "open", 3)
    println(e.into[Stored].transform)
    println(e.into[Narrow].transform)
    println(Duration.between(e.at, e.at.plusSeconds(90)))
