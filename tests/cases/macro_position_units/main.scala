object Main:
  def main(args: Array[String]): Unit =
    val a = "é𝄞"; println(Pos.here)
    println(Pos.here)
    val b = "日本語"; val c = "𝄞𝄞"; println(Pos.here)
    val d = "x𝄞y"; println(Pos.inPair)
    /* 􏠀 */ println(Pos.here)
