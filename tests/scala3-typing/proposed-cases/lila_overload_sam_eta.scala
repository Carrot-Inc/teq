// kamon's TagSet.from is overloaded (scala Map / java.util.Map); lila's mon.scala writes
// `given Conversion[Map[String, Any], TagSet] = TagSet.from`.
import scala.language.implicitConversions

class TagSet(val tags: Map[String, Any]):
  override def toString = tags.map((k, v) => s"$k=$v").mkString("TagSet(", ",", ")")
object TagSet:
  def from(map: Map[String, Any]): TagSet = TagSet(map)
  def from(map: java.util.Map[String, Any]): TagSet = TagSet(Map("java" -> map.size))

given Conversion[Map[String, Any], TagSet] = TagSet.from

def withTags(t: TagSet): String = t.toString

@main def run() =
  println(withTags(Map("name" -> "x", "hit" -> true)))
