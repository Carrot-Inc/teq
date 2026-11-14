// The enclosing trait of the case's other file.
package earlykindprefixwrong

trait Stock { class Crate { def value = 1 }; class Rack[F[G[_]] <: Crate] { def read(x: F[List]): Int = x.value } }
