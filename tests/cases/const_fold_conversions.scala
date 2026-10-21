// `ConstFold.foldUnop`'s conversions through `Constants.intValue`, per source type: a `Float` or a `Double` converted
// to `Int` directly, as the JVM converts it (saturating), never through a `Long`; `toChar` as `intValue.toChar` (the
// review of `4f8b8bb39`'s Convert).
object O {
  val big: 2147483647 = 3e9.toInt
  val bigFloat: 2147483647 = 3e9f.toInt
  val small: -2147483648 = -3e9.toInt
  val char: '￿' = 1e10.toChar
  val long: 9223372036854775807L = 1e19.toLong
}

@main def run(): Unit =
  println(s"${O.big} ${O.bigFloat} ${O.small} ${O.char.toInt} ${O.long}")
