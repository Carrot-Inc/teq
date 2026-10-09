// The members of a refinement as reflection reads them (`Refinement.name`, `.info`, `.parent`):
// a parameterless `def` as a `ByNameType`, a method as its `MethodType` (clause by clause), a
// `val` as its type, a type alias as equal `TypeBounds`; and `Refinement(parent, name, info)`
// makes the same members, as izumi's `Tag` of a structural type rebuilds them.
package app
import mlib.Members

trait S:
  def n: Int

@main def run(): Unit =
  println(Members.of[S { def n: Int }])
  println(Members.of[S { def get(id: Long, k: String): Option[String]; def pair(a: Int)(b: Boolean): Int }])
  println(Members.of[S { val v: String; type T = Int; type U <: AnyVal }])
