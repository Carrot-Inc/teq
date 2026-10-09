// Character's surrogate tests, on JavaScript over its one-character strings.
@main def run(): Unit =
  for c <- List('a', 0xD800.toChar, 0xDBFF.toChar, 0xDC00.toChar, 0xDFFF.toChar, 0xE000.toChar) do
    println(s"${c.toInt.toHexString} ${Character.isSurrogate(c)} ${Character.isHighSurrogate(c)} ${Character.isLowSurrogate(c)}")
