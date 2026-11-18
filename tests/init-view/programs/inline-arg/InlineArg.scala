class InlineArg {
  inline def ignore(x: Int): Int = 1
  val a = ignore(b)
  val b = 2
}
