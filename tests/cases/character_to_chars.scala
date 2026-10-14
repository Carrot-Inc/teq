// `Character.toChars` of a code point: one `Char` in the basic plane, a surrogate pair above it, written into
// a destination from an index; an invalid code point throws `IllegalArgumentException` with the JDK's message.
// The code point helpers it rests on, `isBmpCodePoint`, `isValidCodePoint`, `charCount`, `highSurrogate` and
// `lowSurrogate`, are the JDK's too.
object Main:
  def show(cs: Array[Char]): String = cs.map(c => Integer.toHexString(c.toInt)).mkString(" ")

  def main(args: Array[String]): Unit =
    println(show(Character.toChars(0x41)))
    println(show(Character.toChars(0x1f600)))
    println(Character.toChars(0x1f600).length)
    println(new String(Character.toChars(0x1f600)) == "😀")
    val dst = Array('a', 'b', 'c', 'd')
    println(Character.toChars(0x10ffff, dst, 1))
    println(show(dst))
    println(Character.toChars(0xe9, dst, 3))
    println(show(dst))
    for cp <- List(-1, 0x110000) do
      try Character.toChars(cp)
      catch case e: IllegalArgumentException => println(e.getMessage)
    println(List(Character.isBmpCodePoint(0xffff), Character.isBmpCodePoint(0x10000), Character.isValidCodePoint(0x10ffff), Character.isValidCodePoint(-1)))
    println(List(Character.charCount(0x41), Character.charCount(0x1f600)))
    println(show(Array(Character.highSurrogate(0x1f600), Character.lowSurrogate(0x1f600))))
