// jars: scala-library
// std: scala-library
// The JDK's constructors of a class the std stands for on the JVM, which the std, written for
// JavaScript too, cannot declare: `new PrintWriter(file)` and `new PrintWriter(file, encoding)`
// take a `java.io.File` beside the std's `PrintWriter(writer)`.
import java.io.*

object Main:
  def main(args: Array[String]): Unit =
    val f = File.createTempFile("teq", ".txt")
    Some(new PrintWriter(f)).foreach { p =>
      p.write("hello")
      p.close()
    }
    println(scala.io.Source.fromFile(f).mkString)
    val pw = new PrintWriter(f, "UTF-8")
    pw.println("line")
    pw.close()
    val bw = new BufferedWriter(new FileWriter(f, true))
    bw.write("more")
    bw.close()
    println(scala.io.Source.fromFile(f).getLines().toList)
    f.delete()
