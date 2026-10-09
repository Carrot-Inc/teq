package scala.runtime

import scala.deriving.Mirror

// The values behind the mirrors the compiler synthesizes under `--std=scala-library`
// (src/typer/derive.rs), over scala-library's `Mirror`. A case object's mirror is a Product
// here: scala-library's `Mirror.Singleton` fixes `MirroredMonoType = this.type`, which only
// the object itself can satisfy, and the compiler's refinement says what a mirror mirrors.

final class ProductMirror[T](make: scala.Product => T) extends Mirror.Product:
  type MirroredMonoType = T
  def fromProduct(p: scala.Product): T = make(p)

final class SumMirror[T](ordinalOf: T => Int) extends Mirror.Sum:
  type MirroredMonoType = T
  def ordinal(x: T): Int = ordinalOf(x)

final class SingletonMirror[T](value: T) extends Mirror.Product:
  type MirroredMonoType = T
  def fromProduct(p: scala.Product): T = value
