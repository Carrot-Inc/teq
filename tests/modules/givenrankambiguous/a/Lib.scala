// A given object and a given alias of the same type in one object.
trait T { def n: Int }
object O { given G: T with { def n = 1 }; given a: T = new T { def n = 2 } }
