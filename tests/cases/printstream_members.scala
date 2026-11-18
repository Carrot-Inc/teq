// Every public member of java.io.PrintStream the lean std has, on a stream over a byte array: the prints and
// printlns of each type, printf and format, the appends, the writes, flush, checkError, charset, close; and a
// stream in ISO-8859-1.
import java.io.*
import java.nio.charset.StandardCharsets
@main def run(): Unit =
  val b = new ByteArrayOutputStream()
  val ps = new PrintStream(b, true)
  ps.print(true); ps.print('c'); ps.print(1); ps.print(2L); ps.print(1.5f); ps.print(2.5); ps.print(Array('x', 'y')); ps.print("s")
  ps.print(null: String); ps.print(Some(1): Any); ps.print(null: Any)
  ps.println(); ps.println(false); ps.println('d'); ps.println(3); ps.println(4L); ps.println(3.5f); ps.println(4.5)
  ps.println(Array('z')); ps.println("t"); ps.println(List(1, 2))
  ps.printf("%d-%s|", 7, "f").format("%s+%s|", "a", "b")
  ps.append("app").append('!').append("0123456", 2, 4)
  ps.println()
  ps.write(65); ps.write(Array[Byte](66, 67)); ps.write(Array[Byte](68, 69, 70), 1, 2); ps.writeBytes(Array[Byte](71))
  ps.println()
  ps.flush()
  println(ps.checkError())
  println(ps.charset())
  ps.close()
  println(new String(b.toByteArray, StandardCharsets.UTF_8))
  val latin = new ByteArrayOutputStream()
  val lp = new PrintStream(latin, false, "ISO-8859-1")
  lp.print("café")
  lp.flush()
  println(latin.toByteArray.length + " " + lp.charset())
