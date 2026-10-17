package rcs

import scala.annotation.internal.RuntimeChecked

class Mark extends scala.annotation.StaticAnnotation

// A declared result `@RuntimeChecked` under another annotation, and over one: the pickle's
// `ANNOTATEDtype`s nest, and `Checking.checkIrrefutable`'s `hasAnnotation` recurses over them
// (Types.scala 438-439), so the applications' type carries the exemption either way.
object Api {
  def checked(n: Int): Option[Int] @RuntimeChecked @Mark = Some(n)
  def marked(n: Int): Option[Int] @Mark @RuntimeChecked = Some(n)
}
