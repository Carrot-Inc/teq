// Top-level inline vals, which dotty's Erasure drops with their getters (`isEffectivelyErased`): each read is the
// constant of the val's type, from another package qualified and imported, in a lambda, an interpolation, a default
// argument and a pattern, of an Int, a Long, a String and a Char, beside a constant final val read through an export
// and a plain val.
package a {
  inline val answer = 42
  inline val big = 10000000000L
  inline val name = "teq"
  inline val ch = 'c'
  final val fin = 7
  val plain = 8
}

package b {
  import a.{answer, name}
  export a.fin

  def withDefault(x: Int = a.answer + 1): Int = x

  @main def run(): Unit =
    println(answer)
    println(a.answer + a.big)
    val f = () => answer * 2
    println(f())
    println(s"$name ${a.ch} ${a.fin} ${a.plain}")
    println(withDefault())
    (answer: Int) match
      case a.answer => println("matched")
      case _ => println("no")
    println(fin)
}
