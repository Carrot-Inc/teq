// Character's decimal digits, letters and digits, and numeric values, as JDK 24 answers them: a
// subscript, superscript, Roman or fraction numeral is no decimal digit, an Arabic-Indic or a
// fullwidth digit is one, a fullwidth Latin letter is a digit of a radix above its value, a
// radix outside 2 to 36 has no digits, and getNumericValue tells no value (-1) from a fraction
// (-2). The digit answers are summed over every UTF-16 unit.
@main def run(): Unit =
  for c <- List('₀', '²', 'Ⅰ', '½', '٠', '5', 'a', 'Ａ', 'ａ', 'Ｚ', '\uD800', '\uDFFF', '௰', '〇', 'ↂ', '¼', '൸', 'z', 'Z', '\u0000', '￿', 'Ⓐ', 'ͅ', 'é', '٩', '０') do
    println(c.toInt.toString + " " + c.isDigit + " " + c.isLetterOrDigit + " " + c.isLetter + " " + Character.isDigit(c)
      + " " + Character.isLetter(c) + " " + Character.isLetterOrDigit(c) + " " + Character.getNumericValue(c) + " " + Character.digit(c, 10)
      + " " + Character.digit(c, 36) + " " + c.asDigit)
  val shifted: Char = 8320.toChar
  println(shifted match { case 'a' => "a"; case '￿' => "max"; case n if n.isDigit => "digit"; case _ => "other" })
  println('٣' match { case n if n.isDigit => "digit " + n.asDigit; case _ => "other" })
  println(Character.MIN_RADIX.toString + " " + Character.MAX_RADIX + " " + Character.digit('z', Character.MAX_RADIX))
  for radix <- List(-3, 0, 1, 2, 10, 11, 36, 37, Int.MaxValue) do
    println(radix.toString + " " + Character.digit('1', radix) + " " + Character.digit('a', radix) + " " + Character.digit('٥', radix) + " " + Character.digit('ｚ', radix))
  var digits, decimals, numerics, radixSum = 0L
  var u = 0
  while u < 65536 do
    val c = u.toChar
    if c.isDigit then digits += u
    if Character.isDigit(c) then decimals += u
    numerics = numerics * 31 + Character.getNumericValue(c)
    radixSum = radixSum * 31 + c.asDigit + Character.digit(c, 10) + Character.digit(c, 16) + Character.digit(c, 1)
    u += 1
  println(s"$digits $decimals $numerics $radixSum")
