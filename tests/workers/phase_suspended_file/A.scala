object A:
  inline def choose(inline b: Boolean): Int = inline if b then LocalMacro.run else 0
  val used = choose(true)
  val value: Int = M.next
