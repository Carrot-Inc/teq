package scala.runtime

import scala.deriving.Mirror

// The values behind the mirrors the compiler synthesizes (src/typer/derive.rs): one object per
// class, created when a mirror is first asked for. What a mirror says about its type is in the
// refinement the compiler types the value with, not in the object.

final class ProductMirror[T](make: scala.Product => T) extends Mirror.Product:
  type MirroredMonoType = T
  def fromProduct(p: scala.Product): T = make(p)

final class SumMirror[T](ordinalOf: T => Int) extends Mirror.Sum:
  type MirroredMonoType = T
  def ordinal(x: T): Int = ordinalOf(x)

final class SingletonMirror[T](value: T) extends Mirror.Singleton:
  type MirroredMonoType = T
  def fromProduct(p: scala.Product): T = value
