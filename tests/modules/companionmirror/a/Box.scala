package cba

import scala.deriving.Mirror

// A case class's companion that declares the mirror itself: the declared parent stays, the
// one scalac (and teq's writer) appends is left to teq's typer.
case class Box(n: Int)
object Box extends Mirror.Product:
  type MirroredType = Box
  type MirroredMonoType = Box
  type MirroredElemTypes = Tuple1[Int]
  type MirroredElemLabels = Tuple1["n"]
  type MirroredLabel = "Box"
  def fromProduct(p: Product): Box = new Box(99)
