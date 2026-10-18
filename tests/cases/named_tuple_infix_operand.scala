// A named tuple as an infix operation's right operand (`Desugar.binop`'s `InfixTuple`): a method of one parameter
// takes the tuple (`->` of `ArrowAssoc`, a named tuple's own `==`), one of more parameters its elements as named
// arguments (`Applications.needsTupledDual`), the operation reported by the version (`migrateInfixOp`, silenced).
// teq: --wconf msg=infix.named.arguments.lists:s
object P {
  infix def pair(a: Int, b: String): String = s"$a/$b"
  infix def whole(t: (a: Int, b: String)): String = s"${t.a}:${t.b}"
  def main(args: Array[String]): Unit = {
    println(P pair (b = "x", a = 1))
    println(P whole (a = 2, b = "y"))
    val kv = 3 -> (a = 3, b = "z")
    println(kv._2.b)
    println((a = 1, b = "w") == (a = 1, b = "w"))
  }
}
