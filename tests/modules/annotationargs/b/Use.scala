package aab
import aaa.*
object Use:
  def annotationPath(): Int = Defs.annotationPath[C]
  def annotationCall(): Int = Defs.annotationCall[C]
  def annotationSelect(): Int = Defs.annotationSelect[C]
@main def run(): Unit =
  println(List(Use.annotationPath(), Use.annotationCall(), Use.annotationSelect()))
