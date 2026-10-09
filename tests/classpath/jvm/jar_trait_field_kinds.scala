// jars: scala-library traitfields-lib
// A class mixing in a jar trait holds the trait's fields as scalac's `Mixin` makes them: a val's
// field, a var's getter and setter, a private var's and a private val's under their expanded names
// (`tfl$Kinds$$p_$eq`, which the trait's `$init$` calls: master threw `AbstractMethodError`), a lazy
// val's holder computed once; a chain of jar traits, an object, a program trait over a jar trait,
// a class extending a jar class or a program class that implements the trait already, a trait of
// statements alone, whose `$init$` runs, and a val whose `@targetName` names its accessors.
import tfl.*

class K extends Kinds
object KO extends Kinds:
  println("KO body " + v)
class Chain extends Mid
class Down extends Above
class Again extends K:
  val own: String = "own"
trait Own extends Kinds:
  val mine: Int = { println("init mine"); v + 1 }
  private var count: Int = 0
  def counted(): Int = { count += 1; count }
class Mixed extends Own
class Loud extends Says
class TN extends Named
class H extends Holder

@main def run(): Unit =
  val k = new K
  println(k.show)
  k.w = "w2"
  println(k.show)
  println(k.bumpP() + k.bumpP())
  println(k.bumpR() + k.bumpR())
  println(k.l + k.l)
  println(k.privates)
  println(KO.l + KO.bumpP())
  val ch = new Chain
  println(s"${ch.n} ${ch.m} ${ch.touch()} ${ch.touch()} ${ch.midHidden} ${ch.midHidden}")
  ch.n = 5
  println(ch.touch())
  val d = new Down
  println(d.above + " " + d.bumpP() + " " + d.l + d.l)
  val a = new Again
  println(a.own + " " + a.bumpP() + " " + a.privates)
  val m = new Mixed
  println(s"${m.mine} ${m.counted()} ${m.counted()} ${m.bumpP()} ${m.l}")
  println(new Loud().hello)
  val tn = new TN
  println((tn.x, tn.twice))
  val h = new H
  println(h.Inner.z + h.Inner.z)
