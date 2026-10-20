object A:
  val value: Int = M.transparentNext
  inline def choose(inline b: Boolean): Int = inline if b then LocalMacro.run else 0
  val used = choose(true)
