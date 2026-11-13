// A class whose type parameter takes a type constructor of a given kind (`F[G[_]]`), defined in another file
// and first applied here: the parameter's kind is read in the class's own file, and `K`, a type that takes a
// constructor, is an argument of that kind. scalac 3.8.4 accepts it and prints `ok`.
package crosshk
type K[F[_]] = Int
class Applied extends Later[K]
@main def main(): Unit = println(if new Applied().tag == 1 then "ok" else "no")
