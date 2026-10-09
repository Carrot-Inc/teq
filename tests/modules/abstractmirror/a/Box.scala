package ama

import scala.deriving.Mirror

// An abstract case class, which takes no synthesized product mirror, with a companion that
// declares one: the declared parent is the only one and stays.
abstract case class Box(n: Int)
object Box extends Mirror.Product:
  type MirroredType = Int
  type MirroredMonoType = Int
  type MirroredElemTypes = EmptyTuple
  type MirroredElemLabels = EmptyTuple
  type MirroredLabel = "Box"
  def fromProduct(p: Product): Int = 42
