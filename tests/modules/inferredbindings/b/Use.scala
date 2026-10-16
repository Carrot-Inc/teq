package iib
import iia.*
object Use:
  def inferredPat(): Int = Defs.inferredPat[C]
  def refinedMatch(): Int = Defs.refinedMatch[C]
  def targWritten(): Int = Defs.targWritten[C]
  def targInferred(): Int = Defs.targInferred[C]
  def lambdaInferred(): Int = Defs.lambdaInferred[C]
  def mapIdentity(): Int = Defs.mapIdentity[C]
@main def run(): Unit =
  println(List(Use.inferredPat(), Use.refinedMatch(), Use.targWritten(), Use.targInferred(), Use.lambdaInferred(), Use.mapIdentity()))
