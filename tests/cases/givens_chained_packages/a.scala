//> using scala 3.8.4
package a
trait T { def n: String }
trait U { def n: String }
given aGiven: T with { def n = "a" }
given aU: U with { def n = "a.u" }
