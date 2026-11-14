// An inner class whose higher-kinded parameter has a written bound, applied through a subclass of the enclosing
// trait in the file typed before its own, to an argument that is not within the bound: refused. scalac 3.8.4:
// E057, "Type argument Tin does not conform to upper bound [G[_$1]] =>> Depot.Crate". The sentence here is the
// kind's until an inner class's bound is seen from the application's prefix; what is held is the refusal, without
// which the program is built and `read` calls a member its argument does not have.
// expect: A.scala:11:53: error: type argument Tin does not
// expect: 1 error found
package earlykindprefixwrong

class Tin[G[_]]
object Depot extends Stock { class Use extends Rack[Tin] { def go = read(new Tin[List]) } }
@main def main(): Unit = println(new Depot.Use().go)
