package rci

// A transparent inline wrapper around `.runtimeChecked`: its expansion's type is its body's,
// `x.type @RuntimeChecked`, the annotation pickled with the body and read back downstream.
object Checked {
  transparent inline def checked(x: Option[Int]) = x.runtimeChecked
}
