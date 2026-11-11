package mp

opaque type OT = Int
object OT:
  def mk(i: Int): OT = i
  extension (o: OT) def raw: Int = o

object Misc:
  inline def m2(o: OT): Int = (o: OT).raw
