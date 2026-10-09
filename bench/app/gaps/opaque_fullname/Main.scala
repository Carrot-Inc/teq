package probe.opq

opaque type DomeId = Long
object DomeId extends LongKey[DomeId]

opaque type SiteId = Long
object SiteId extends LongKey[SiteId]:
  def first: SiteId = 1L

def show[K](k: K)(using key: Key[K]): String = s"${key.keyName}#${k.raw}"

object Main:
  def main(args: Array[String]): Unit =
    val d = DomeId(7L)
    println(show(d))
    println(show(SiteId.first))
    println(Key[DomeId].wrap(9L).raw)
    println(summon[Key[SiteId]].keyName)
