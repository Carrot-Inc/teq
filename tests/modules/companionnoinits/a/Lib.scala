package cni

// Case classes of an object, the companions `Desugar.classDef` makes them running no initialiser,
// which a downstream module reads from the products as `TreeUnpickler.indexStats` finds it (a
// `derives` clause's given, which `Namer` leaves out of the body it judges, too); and one whose
// written companion runs a statement.
object Models:
  final case class Id(value: String)
  final case class Pair(a: Int, b: String)
  final case class Tagged(v: Int) derives CanEqual
  final case class Loud(v: Int)
  object Loud:
    print("loud ")
