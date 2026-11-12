package fix

object A:
  val otherFile: Option[Int] = None
  def otherFileDef: Option[Int] = None

class ABase:
  val otherFileInherited: Option[Int] = None

type OI = Option[Int]
opaque type Opq = Option[Int]
object Opq:
  def apply(): Opq = None
  extension (o: Opq) def under: Option[Int] = o
