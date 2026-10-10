// The givens an extension is selected from stand at the search's levels (`ContextualImplicits`): each package
// clause one, the file's imports at the innermost, a same-named given of a level hiding an import of its name
// (`combineEligibles`), and a nested package's import a level nearer than the outer package's givens; an import
// in a block of the method stands at the level of the method's givens.
package lvl

trait Ops { extension (x: Int) def label: String }

given ops: Ops with
  extension (x: Int) def label = "package"

object A:
  given ops: Ops with
    extension (x: Int) def label = "imported"

object B:
  given nestedOps: Ops with
    extension (x: Int) def label = "nested import"

object C:
  given local: Ops with
    extension (x: Int) def label = "imported local"
