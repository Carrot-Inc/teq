// The oracle's capture-checked fixture (README.md): `inferred`'s result is `Object^{x}`, which 3.8.4 pickles as
// ANNOTATEDtype over the tree `new retains[..]` under -source:3.8 and over the type `retains[..]` under -source:3.9
// (TreePickler.scala 285-299, SourceVersion.enablesCompactAnnotation).
package repr.cc

import language.experimental.captureChecking

object Cap:
  def keep(x: Object^): Object^{x} = x
  private def inferred(x: Object^) = keep(x)
  def use(x: Object^): Object^{x} = inferred(x)
