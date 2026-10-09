// jars: scala-library scala2-lib
// Scala 2.13 traits whose names hold a `$` the source writes keep the storage a class mixing them in takes from their
// class files: a top-level `` `A$B` ``'s private lazy val computed once (its pickle's class named by its owners, not
// split at the `$`), and a trait nested in `` object `C$D` ``, whose pickle is on `C$D.class`, its `$init$`, private
// var, object and private lazy val. The first was once computed at every read and the second threw
// `AbstractMethodError`.
import dollars.*

class AB extends `A$B`
class CD extends `C$D`.T

@main def run(): Unit =
  val ab = AB()
  println(ab.read + ab.read)
  val cd = CD()
  println(cd.next + cd.next)
  println((cd.cell eq cd.cell, cd.cell.owner eq cd))
