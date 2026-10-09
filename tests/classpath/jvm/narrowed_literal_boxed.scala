// jars: scala-library
// std: scala-library
// An integer literal the typer narrows to the `Byte` or `Short` expected keeps that width when it is
// boxed again (`(1: Byte): Any` is a `java.lang.Byte`), as scalac boxes it.
object Main:
  def f(a: Any): String = a.getClass.getName
  def main(args: Array[String]): Unit =
    println(((1: Byte): Any).getClass.getName)
    println(f((1: Byte): Any))
    val y: Byte = 1
    println(((y: Byte): Any).getClass.getName)
    println((y: Any).getClass.getName)
    println(((1: Short): Any).getClass.getName)
