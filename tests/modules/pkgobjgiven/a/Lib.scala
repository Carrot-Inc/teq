package pga

// A package object's given object and given class: teq's pickles and class files hold a package object's members
// in the file's `Lib$package` (scalac's `package$`), the givens' classes inside it, where a downstream finds them.
trait TC:
  def name: String

package object inner:
  given tc: TC with
    def name = "tc"
  given listTC[A](using t: TC): TC with
    def name = "list " + t.name
