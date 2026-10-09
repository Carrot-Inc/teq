// jars: scala-library scala2-lib
// A class mixing in Scala 2.13 traits holds what their transcoded pickles leave out, from the traits' class files
// as scalac 3's `Mixin` does for a `Scala2x` trait (`traitInits`, 286-295): a private var's field and its expanded
// accessors, a private lazy val's holder computed once (a private def of the same shape runs at every call), an
// object's holder made once with the instance as its outer reference, and the trait's `$init$` where its
// initialisers are all private or its body is statements alone.
// master threw `AbstractMethodError` at `Logging$$log_` and ran neither trait's `$init$`.
import scala2lib.*

class Service extends Logging with Says
class T2 extends Ticker with OnlyPrivate
object TO extends Ticker

@main def run(): Unit =
  val s = Service()
  println(s.logInfo("a"))
  println(s.logInfo("b"))
  val t = T2()
  println(t.tick + t.tick)
  println((t.cell eq t.cell, t.cell.owner eq t))
  println(t.reveal + t.reveal)
  println(TO.tick)
  println(TO.cell.owner eq TO)
