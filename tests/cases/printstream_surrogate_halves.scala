// A half of a surrogate pair printed alone: a high half is held until the next text (the pair is encoded whole
// when the low half follows), written as the charset's replacement before whatever else follows, and at close;
// a low half alone is the replacement at once. The expected lines are scalac's on the JDK.
import java.io.*
object Probe:
  def bytes(b: ByteArrayOutputStream): String = b.toByteArray.map(_ & 255).mkString(",")
  def main(args: Array[String]): Unit =
    val b1 = new ByteArrayOutputStream; val p1 = new PrintStream(b1, false, "UTF-8")
    p1.print('\uD83D'); p1.print('x'); p1.flush(); println("high then a letter: " + bytes(b1))
    val b2 = new ByteArrayOutputStream; val p2 = new PrintStream(b2, false, "UTF-8")
    p2.print('\uD83D'); p2.close(); println("high then close: " + bytes(b2))
    val b3 = new ByteArrayOutputStream; val p3 = new PrintStream(b3, false, "UTF-8")
    p3.print('\uDE00'); p3.flush(); println("a low alone: " + bytes(b3))
    val b4 = new ByteArrayOutputStream; val p4 = new PrintStream(b4, false, "UTF-8")
    p4.print("a\uD83D"); p4.println(); p4.flush(); println("high then a line's end: " + bytes(b4))
    val b5 = new ByteArrayOutputStream; val p5 = new PrintStream(b5, false, "UTF-16")
    p5.print(""); p5.print("A"); p5.flush(); p5.print("B"); p5.flush(); println("UTF-16 with an empty print first: " + bytes(b5))
    val b6 = new ByteArrayOutputStream; val p6 = new PrintStream(b6, false, "UTF-16LE")
    p6.print("A"); p6.print("B"); p6.flush(); println("UTF-16LE: " + bytes(b6))
    val b7 = new ByteArrayOutputStream; val p7 = new PrintStream(b7, false, "US-ASCII")
    p7.print("café"); p7.flush(); println("US-ASCII of an e acute: " + bytes(b7))
