package pga

// A package object's given object and given class: members of the object `pga.inner.package`, the givens' classes
// inside its `package$`, where a downstream finds them.
trait TC:
  def name: String

package object inner:
  given tc: TC with
    def name = "tc"
  given listTC[A](using t: TC): TC with
    def name = "list " + t.name
