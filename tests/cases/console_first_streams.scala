// Console.out and Console.err are the streams of Console's first use: a System.setOut or setErr after it does not
// move them (the reference binds both as the object is made), while System.out, System.err and a withOut over
// System.out see the new streams. The expected lines are scalac's on the JDK.
import java.io.{ByteArrayOutputStream, PrintStream}
object Probe:
  def main(args: Array[String]): Unit =
    val first = new ByteArrayOutputStream
    Console.withOut(new PrintStream(first)) { println("Console's first use") }
    val out0 = System.out
    val err0 = System.err
    val outs = new ByteArrayOutputStream
    val errs = new ByteArrayOutputStream
    System.setOut(new PrintStream(outs, true))
    System.setErr(new PrintStream(errs, true))
    Console.out.println("Console.out after setOut")
    Console.out.flush()
    System.out.println("System.out after setOut")
    System.err.println("System.err after setErr")
    Console.withOut(System.out) { println("withOut(System.out) after setOut") }
    val sameOut = Console.out eq out0
    val sameErr = Console.err eq err0
    val newOut = Console.out eq System.out
    val newErr = Console.err eq System.err
    System.setOut(out0)
    System.setErr(err0)
    println("captured by the new out: " + outs.toString("UTF-8").replace("\n", "|"))
    println("captured by the new err: " + errs.toString("UTF-8").replace("\n", "|"))
    println("Console.out is the first stream: " + sameOut + ", the new one: " + newOut)
    println("Console.err is the first stream: " + sameErr + ", the new one: " + newErr)
    println("first: " + first.toString("UTF-8").replace("\n", "|"))
