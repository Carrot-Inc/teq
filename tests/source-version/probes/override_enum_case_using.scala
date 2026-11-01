enum E(val s: Int):
  case C(using override val s: Int) extends E(s)
