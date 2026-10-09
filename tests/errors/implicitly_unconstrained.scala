// expect: 6:22: error: no given instance of type T was found for parameter x
// expect: 1 error found
// After Scala 3's tests/neg/i7745.scala: a search for a type nothing constrains finds nothing.
trait F[x]
implicit def foo[f[_], y, x <: f[y]](implicit ev: F[y]): F[x] = ???
val test = implicitly
