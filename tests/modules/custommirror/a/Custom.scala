package cma

import scala.deriving.Mirror

// A mirror the source declares stays a parent of a product's object; only those the compiler
// synthesizes for a case object or a companion are left to teq's typer.
object Custom extends Mirror.Product:
  type MirroredType = Int
  type MirroredMonoType = Int
  type MirroredElemTypes = EmptyTuple
  type MirroredElemLabels = EmptyTuple
  type MirroredLabel = "Custom"
  def fromProduct(p: Product): Int = 42
