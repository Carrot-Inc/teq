// `trim` takes every character up to U+0020 off both ends, `strip`, `stripLeading`,
// `stripTrailing` and `isBlank` the white space of `Character.isWhitespace`; neither is
// JavaScript's `trim`, which takes the no-break spaces and U+FEFF and leaves the controls.
object Main:
  def main(args: Array[String]): Unit =
    val chars = List(0x0, 0x1, 0x9, 0xA, 0xD, 0x1C, 0x1F, 0x20, 0x7F, 0x85, 0xA0, 0x1680, 0x180E,
      0x2000, 0x2003, 0x2007, 0x200B, 0x2028, 0x2029, 0x202F, 0x205F, 0x3000, 0xFEFF)
    for c <- chars do
      val w = c.toChar.toString
      val s = w + "a" + w + "b" + w
      val cells = List(s.trim.length, s.strip.length, s.stripLeading.length, s.stripTrailing.length,
        (w + w).isBlank, (w + w).trim.isEmpty, w.trim.length)
      println(f"$c%04X " + cells.mkString(" "))
    println("[" + " \u0000 x \u0000 ".trim + "]")
    println("".trim.isEmpty)
    println("   ".strip.isEmpty)
    println("\u0000".isBlank)
