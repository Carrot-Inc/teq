package uib
import uia.*
object Use:
  def written(): Int = Defs.written[C]
  def inferred(): Int = Defs.inferred[C]
  def inferredCall(): Int = Defs.inferredCall[C]
  def tapply(): Int = Defs.tapply[C]
  def ascribed(): Int = Defs.ascribed[C]
  def layered(): Int = Defs.layered[C]
  def fresh(): Int = Defs.fresh[C]
  def refined(): Int = Defs.refined[C]
  def matched(): Int = Defs.matched[C]
  def singleton(): Int = Defs.singleton[C]
  def nested(): Int = Defs.nested[C]
  def nestedCall(): Int = Defs.nestedCall[C]
  def localdef(): Int = Defs.localdef[C]
  def byname(): Int = Defs.byname[C]
  def alias(): Int = Defs.alias[C]
  def typeAnnotated(): Int = Defs.typeAnnotated[C]
  def termAnnotated(): Int = Defs.termAnnotated[C]
  def annotatedType(): Int = Defs.annotatedType[C]
  def variable(): Int = Defs.variable[C]
  def lambda(): Int = Defs.lambda[C]
  def returned(): Any = Defs.returned[C]
  def returnedInferred(): Any = Defs.returnedInferred[C]
  def returnedTransparent(): Any = Defs.returnedTransparent[C]
  def dependent(): Int = Defs.dependent[C](None)
  def cast(): Int = Defs.cast[C]
  def nothing(): Int = Defs.nothing[C]
@main def run(): Unit =
  println(List(Use.written(), Use.inferred(), Use.inferredCall(), Use.tapply(), Use.ascribed(), Use.layered(), Use.fresh(), Use.refined(), Use.matched(), Use.singleton()))
  println(List(Use.nested(), Use.nestedCall(), Use.localdef(), Use.byname(), Use.alias(), Use.typeAnnotated(), Use.termAnnotated(), Use.annotatedType(), Use.variable(), Use.lambda()))
  println(List(Use.returned(), Use.returnedInferred(), Use.returnedTransparent(), Use.dependent(), Use.cast(), Use.nothing()))
