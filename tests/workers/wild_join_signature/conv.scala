import scala.language.implicitConversions
object Convs:
  implicit def fromList(l: List[AttrPair[Builder, ?]]): Int = l.size
  implicit def fromSeq(l: Seq[AttrPair[Builder, ?]]): Int = l.size + 100
