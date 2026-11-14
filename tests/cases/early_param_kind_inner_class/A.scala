// A class nested in a trait or in a class, whose higher-kinded parameter's bound names a member or a type parameter
// of the enclosing one, applied by its simple name in a subclass of the enclosing one, in the file typed before
// its own: the bound is the enclosing instance's and is seen from the subclass, so the application is accepted as
// it was before the parameter had its kind from the start. scalac 3.8.4 prints `201 202`.
package earlykindprefix

object Shelf extends Stock { class Use extends Rack[Crate] { def value = 201 } }
class Depot extends Yard[Load] { class Bay extends Slot[LoadOf] { def value = 202 } }

@main def main(): Unit =
  val depot = new Depot
  println(s"${new Shelf.Use().value} ${new depot.Bay().value}")
