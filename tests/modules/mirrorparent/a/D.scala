package mpa

import scala.deriving.Mirror

// A case class whose companion declares `Mirror.Product` itself: scalac's `SyntheticMembers`
// adds the synthesized parent beside the declared one in the typed tree, so the pickle holds
// both, and its class file implements the interface once; the mirror a summon synthesizes is
// that companion, its declared `fromProduct` the one that answers.
case class D(x: Int)

object D extends Mirror.Product:
  type MirroredMonoType = D
  def fromProduct(p: Product): D = new D(p.productElement(0).asInstanceOf[Int] * 10)

object Lib:
  val mirror = summon[Mirror.ProductOf[D]]
