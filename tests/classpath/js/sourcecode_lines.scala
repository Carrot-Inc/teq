// jars: sourcecode
// sourcecode's own macro, run in the interpreter from the jar's TASTy.
// The line an implicit `sourcecode.Line` records for a call that spans several lines: the
// line of the call itself, as sourcecode's macro takes it from the expansion's position.
//> using dep com.lihaoyi::sourcecode:0.4.2
package demo.lines

object Site:
  def here(implicit line: sourcecode.Line): Int = line.value
  def fc(body: => String)(implicit name: sourcecode.Name, line: sourcecode.Line): String =
    name.value + "@" + line.value + ": " + body

object Main:
  def main(args: Array[String]): Unit =
    val a = Site.fc(
      "multi" +
        "line"
    )
    println(a)
    val b = Site.fc {
      "block"
    }
    println(b)
    println(
      Site.here
    )
    val c =
      Site.fc("x")
    println(c)
    val d = Site.fc(
      "y")(using sourcecode.Name("named"), sourcecode.Line(99))
    println(d)
    val chained = Site
      .fc("z")
    println(chained)
