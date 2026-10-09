package errs

/** Imports in the bodies of classes that name nothing, resolved when the alias beside each is:
  * a retype resolves the imports of a class again. */
object Inner {
  import errs.nowhere.X
  type T = Int
  val text: String = "a"

  class Nested {
    import errs.elsewhere.Y
    type U = Int
    def label: String = "n"
  }
}

given innerShown: Shown[Int] with
  import errs.nothere.Z
  type V = Int
  def show(a: Int): String = "s"
