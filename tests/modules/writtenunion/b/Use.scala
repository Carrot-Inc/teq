package wub
import wua.*
object Use:
  def unionVal(): Int = Defs.unionVal[C]
  def unionAscribed(): Int = Defs.unionAscribed[C]
  def unionTarg(): Int = Defs.unionTarg[C]
  def interVal(): Int = Defs.interVal[C]
  def aliasMatch(): Int = Defs.aliasMatch[C]
@main def run(): Unit =
  println(List(Use.unionVal(), Use.unionAscribed(), Use.unionTarg(), Use.interVal(), Use.aliasMatch()))
