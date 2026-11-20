// A Java source's constants as the reference reads them: an interface's fields static and final, one literal each, the
// escapes and the text block decoded by the scanner (a lone surrogate's escape kept as its one UTF-16 unit, a pair's as
// its character), the literal converted to the field's written type.
object Main {
  val lowCode: 56832 = C.LOW_CODE
  val size: 200 = K.SIZE
  val name: "k" = K.NAME
  val big: 9000000000L = C.BIG
  val letter: 'q' = C.LETTER
  val half: 0.5 = C.HALF
  val yes: true = C.YES
  val negative: -12 = C.NEGATIVE
  val octal: 15 = C.OCTAL
  val hex: 255 = C.HEX
  val small: Byte = C.SMALL
  def main(args: Array[String]): Unit = {
    println(List(size, name, big, letter, half, yes, negative, octal, hex, small).mkString(" "))
    println(C.THIRD > 0.33f)
    println(C.NUL.length + " " + C.WIDE.codePointCount(0, C.WIDE.length))
    println(C.LONE.map(c => Integer.toHexString(c.toInt)).mkString(" ") + " " + Integer.toHexString(C.LOW.toInt) + " " + lowCode + " " + C.PAIR.codePointCount(0, C.PAIR.length))
    print(C.BLOCK)
  }
}
