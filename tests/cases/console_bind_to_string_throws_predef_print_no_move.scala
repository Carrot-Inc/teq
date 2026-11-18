// The first value-taking print (predefPrint) whose argument's toString throws, System.setOut after it: Console binds
// its streams as the print is entered, after the argument is evaluated and before its text is made (scala.Console
// 128-129, 248-269); then which streams Console.out and System.out are, and each entry point once through a logging
// subclass. The expected lines are scalac's on the JDK.
import java.io.{ByteArrayOutputStream, PrintStream}

trait Marked:
  def tag: String
class Log(val name: String) extends PrintStream(new ByteArrayOutputStream(), true):
  private def d(x: Any): String = x match
    case null => "null"
    case s: String => "S'" + s + "'"
    case m: Marked => "M'" + m.tag + "'"
    case o => "O"
  private def note(s: String): Unit = M.log.append(name).append('.').append(s).append(';')
  override def print(s: String): Unit = note("print(String " + d(s) + ")")
  override def print(x: Any): Unit = note("print(Object " + d(x) + ")")
  override def println(): Unit = note("println()")
  override def println(s: String): Unit = note("println(String " + d(s) + ")")
  override def println(x: Any): Unit = note("println(Object " + d(x) + ")")
  override def flush(): Unit = note("flush()")

object Mov:
  java.lang.System.setOut(M.B)
  val x: String = "mov"
class Mover extends Marked:
  def tag = "mover"
  override def toString: String = { java.lang.System.setOut(M.B); "mover-text" }
class Bad extends Marked:
  def tag = "bad"
  override def toString: String = throw new RuntimeException("bad")
class MoverP extends Marked:
  def tag = "moverp"
  override def toString: String = { java.lang.System.setOut(M.B); println("inner"); "moverp-text" }

object M:
  val log = new java.lang.StringBuilder
  val orig: PrintStream = java.lang.System.out
  val A = new Log("A")
  val B = new Log("B")
  def mark(s: String): Unit = orig.print("#" + s + ":")
  def done(): Unit =
    orig.println(" log=[" + log.toString + "]")
    log.setLength(0)
  def who(p: PrintStream): String = if p eq orig then "orig" else p match
    case l: Log => l.name
    case _ => "other"
  def main(args: Array[String]): Unit =
    mark("trigger")
    try { print(new Bad) } catch { case _: RuntimeException => log.append("thrown;") }
    done()
    java.lang.System.setOut(A)
    orig.println("Console.out=" + who(scala.Console.out) + " System.out=" + who(java.lang.System.out))
    mark("Predef.print"); print("p"); done()
    mark("Predef.println"); println("l"); done()
    mark("Predef.println()"); println(); done()
    mark("Console.print"); scala.Console.print("cp"); done()
    mark("Console.println"); scala.Console.println("cl"); done()
    mark("Console.println()"); scala.Console.println(); done()
    mark("Console.out.print"); scala.Console.out.print("co"); done()
    mark("System.out.print"); java.lang.System.out.print("so"); done()
    java.lang.System.setOut(orig)
    orig.println("end")
