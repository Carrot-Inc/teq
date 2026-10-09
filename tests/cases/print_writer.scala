// java.io's StringWriter and PrintWriter, and a Throwable printed through one without its
// frames, as izumi-reflect's `summonTag` reports a failed search.
import java.io.{PrintWriter, StringWriter}

object Main:
  def main(args: Array[String]): Unit =
    val sw = new StringWriter()
    val pw = new PrintWriter(sw)
    pw.print("a")
    pw.println(1)
    pw.write("xy")
    pw.println()
    pw.flush()
    println(sw.toString)
    val report = new StringWriter()
    new RuntimeException("outer", new IllegalStateException("inner")).printStackTrace(new PrintWriter(report))
    println(report.toString.linesIterator.filterNot(_.startsWith("\t")).mkString("|"))
    val w = new StringWriter()
    w.append("p").append('q')
    println(w)
    println("a\n\nb\n".linesIterator.toList)
    println(("".linesIterator.size, "\n".linesIterator.toList, "x\r\ny".linesIterator.toList))
