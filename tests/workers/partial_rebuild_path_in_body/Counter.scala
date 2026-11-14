package rebuilt

class Sub(t: String) extends Base(t):
  transparent inline def pick: Int =
    val got = List.empty[this.tag.type]
    got.size
