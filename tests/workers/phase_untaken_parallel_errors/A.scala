object A:
  inline def choose(inline yes: Boolean): Int = inline if yes then M.value else 0
  val value: Int = choose(false)
  val bad: Int = scala.compiletime.error("A_ERROR")
