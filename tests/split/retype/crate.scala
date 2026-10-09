package retype

import scala.annotation.unchecked.uncheckedVariance

def crateLabel(n: Int): String =
  val text = "crate"
  text + n

/** A covariant parameter in a parameter's type, which the annotation allows: what it resolved
  * to is kept by the type expression, which an edit above moves. */
class Crate[+A](val items: List[A]):
  def has(x: A @uncheckedVariance): Boolean = items.contains(x)
  def label: String = "crate"
