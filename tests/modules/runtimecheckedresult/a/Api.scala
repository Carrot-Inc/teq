package rcr

import scala.annotation.internal.RuntimeChecked

// An ordinary method's declared result annotated `@RuntimeChecked`: its applications' type carries
// the annotation, pickled with the result type (TreePickler's `ANNOTATEDtype`) and read back.
object Api {
  def checked(n: Int): Option[Int] @RuntimeChecked = Some(n)
}
