package rca

import scala.annotation.internal.RuntimeChecked

// An alias of an annotated type is dotty's `TypeRef`, whose top carries no annotation (`hasAnnotation`
// reads a type undealiased, Types.scala 438-440): the component `Tuple1[Checked]` exempts nothing, and
// the published signature carries no annotation either.
type Checked = Option[Int] @RuntimeChecked

object E {
  def unapply(x: Int): Tuple1[Checked] = Tuple1(Some(x))
}
