// Console.withOut over a stream of the program's own receives what the prelude's println and print write, what
// Console's own do, and not what System.out prints.
import java.io.{ByteArrayOutputStream, PrintStream}
@main def run(): Unit =
  val baos = new ByteArrayOutputStream()
  val ps = new PrintStream(baos)
  Console.withOut(ps) {
    println("captured line")
    print("no newline")
    println()
    Console.println("console line")
    println(42)
    println("café ✓")
    System.out.println("to the process")
  }
  ps.flush()
  val bytes = baos.toByteArray
  val text = new String(bytes, "UTF-8")
  println("bytes " + bytes.length + " chars " + text.length)
  text.split("\n").foreach(l => println("[" + l + "]"))
  val b2 = new ByteArrayOutputStream()
  Console.withOut(new PrintStream(b2)) {
    println("outer")
    Console.withOut(System.out) { println("back to the process") }
    println("outer again")
  }
  println(new String(b2.toByteArray, "UTF-8").trim.replace("\n", "|"))
  val b3 = new ByteArrayOutputStream()
  Console.withErr(new PrintStream(b3)) { Console.err.println("err line") }
  println(new String(b3.toByteArray, "UTF-8").trim)
  println("after")
