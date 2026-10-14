// An exported object's forwarder applied to type arguments is its `apply`'s application, as dotty's
// `typedTypeApply` inserts `apply` on a value that is no polymorphic method: `Enumerated[Feature]`
// through a package's export of the trait and its companion (the corpus's `meridian.core` export).
package app

object enums:
  trait Enumerated[E]:
    def values: List[E]
  object Enumerated:
    def apply[E](using e: Enumerated[E]): e.type = e

export enums.Enumerated

enum Feature:
  case A, B

object Feature:
  given Enumerated[Feature] with
    def values: List[Feature] = Feature.values.toList

@main def run(): Unit =
  println(Enumerated[Feature].values)
  println(Enumerated.apply[Feature].values.size)
