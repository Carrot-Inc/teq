// A class parameter bounded by a constructor indirectly (`G <: IndP#F`, no lambda in the pickle), for the classpath
// case loaded_indirect_kind: a client reading these pickles gives the class its kind (the review of `b8eb8842a`), and
// the opaque type its definition's parameters' kinds (the review of `4fd7f3293`).
package fix.indirect

trait IndP:
  type F[X]
class IndC[G <: IndP#F]
object IndLib:
  opaque type IndO[G <: IndP#F] = Any
