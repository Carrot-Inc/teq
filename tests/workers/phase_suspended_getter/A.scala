object A:
  val used = LocalMacro.run
  case class C(a: Int, b: Int = 1)
  object C
  val value: Int = M.next
