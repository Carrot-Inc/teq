package gsa

// A top-level given object, summoned through a wildcard and a given import over the products.
trait T { def n: Int }
given top: T with { def n = 42 }
