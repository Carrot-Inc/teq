package pcua

// A curried polymorphic context function, chimney's `applyFieldNameTypes` shape: its literal is
// typed against the `apply` (dotty's `typedPolyFunctionValue`), which the restriction on curried
// dependent context functions does not concern.
object P:
  def both[Out](f: [A <: AnyVal, B] => A ?=> B ?=> Out)(using n: Int, s: String): Out = f[Int, String]
