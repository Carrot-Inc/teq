// jars: scala-library
// std: lean scala-library
// `java.util.Date.from(instant)` and `toInstant`, the JDK's statics of a class the std stands
// for on the JVM, reached through the std's object of the class.
import java.time.Instant

object Main:
  def main(args: Array[String]): Unit =
    val d = java.util.Date.from(Instant.ofEpochMilli(1000))
    println(d.getTime())
    println(d.toInstant())
