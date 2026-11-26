// A given object and a given alias of a subtype in one object.
trait T { def n: Int }
trait S extends T
object O { given G: T with { def n = 1 }; given a: S = new S { def n = 2 } }
