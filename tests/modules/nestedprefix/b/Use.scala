package npb

import npa.*

object Use:
  def main(args: Array[String]): Unit =
    val i: API.a.Item = API.make
    println(i.value)
    println(summon[API.b.Item].value)
    println(summon[API.a.Item] eq API.a.item)
    val s: String = API.gen.get
    println(s)
    val p: O#Item = API.make
    println(p.value + 1)
