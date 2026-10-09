// jars: sourcecode
// sourcecode's macros against the real jar: Name, Line, FileName, Enclosing, Pkg, Text and
// Args all come from the library's own macros, which run in the interpreter from the TASTy
// bodies at compile time, whatever the target.
// The expectations are scalac's with the same jar.
//> using dep com.lihaoyi::sourcecode:0.4.2
package demo.app

object Config:
  def label(implicit name: sourcecode.Name, line: sourcecode.Line, file: sourcecode.FileName): String =
    s"${name.value} at ${file.value}:${line.value}"
  val port = label
  lazy val host = label

class Greeter(who: String):
  def greet(): String =
    val here = sourcecode.Enclosing()
    s"$here says hello to $who"

object Main:
  def describe(x: Int, y: String)(implicit args: sourcecode.Args): String =
    args.value.map(_.map(a => s"${a.source}=${a.value}").mkString(", ")).mkString("; ")
  def call(x: Int, y: String): String = describe(x, y)

  def main(args: Array[String]): Unit =
    println(Config.port)
    println(Config.host)
    println(new Greeter("world").greet())
    println(sourcecode.Pkg())
    println(sourcecode.FullName())
    println(sourcecode.Name())
    val t = sourcecode.Text(List(1, 2, 3).map(_ * 2))
    println(s"${t.source} -> ${t.value}")
    println(call(40 + 2, "z" * 3))
    println(implicitly[sourcecode.File].value.endsWith("sourcecode_macros.scala"))
