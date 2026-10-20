val top: Int = M.next
object O:
  val inObject: Int = M.next
def topDef: Int = M.next
class C:
  val inClass: Int = M.next
opaque type T = Int
object T:
  val inCompanion: Int = M.next
given g: Long = M.next.toLong
object P:
  val inLast: Int = M.next
@main def run(): Unit =
  println(s"$top,${O.inObject},$topDef,${new C().inClass},${T.inCompanion},${summon[Long]},${P.inLast}")
