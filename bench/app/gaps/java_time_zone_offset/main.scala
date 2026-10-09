// jars: scala-library
// java.time under teq: without a class path the JDK is not there at all (`java is a package, not
// a value`); with one (which opens the JDK's ct.sym) the classes are read but the static field
// `ZoneOffset.UTC` is not seen (`value UTC is not a member of ZoneOffset`, then `the constructor
// of ZoneOffset is private`), for JavaScript as under --target jvm. It is the one thing the
// --java-time variant of the corpus stops on. scalac prints `1970-01-01`.
type Instant = java.time.Instant
object Instant:
  def ofEpochSecond(s: Long): Instant = java.time.Instant.ofEpochSecond(s)
object Main:
  def main(args: Array[String]): Unit =
    println(Instant.ofEpochSecond(3600L).atOffset(java.time.ZoneOffset.UTC).toLocalDate)
