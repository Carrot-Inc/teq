// jars: scala-library javafixdir
// A class directory on the classpath.
object Main:
  def main(args: Array[String]): Unit =
    println(fix.Named.join("-", "a", "b"))
    println(new fix.Generics[Integer](Integer.valueOf(1)).value)
