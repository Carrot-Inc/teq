//> using platform js
// The conditions scalac folds away, whose branch carries a chain of `+` on, and the ones it keeps.
class Loud(s: String):
  override def toString: String =
    println("  render " + s)
    s

def tail(): String =
  println("  tail")
  "!"

object Consts:
  println("  init Consts")
  final val T = true
  inline val I = true
  val V = true

object Bare:
  final val T = true

class C:
  final val yes = true
  inline val iyes = true
  val plain = true
  def own(l: Any): String = (if yes then "" + l else "") + tail()
  def ownThis(l: Any): String = (if this.yes then "" + l else "") + tail()

trait Flags:
  final val on = true

class WithFlags extends Flags

class Holder:
  val c = new C

type Bool = Boolean

def getC(): C =
  println("  condition")
  new C

@main def main(): Unit =
  val l = new Loud("x")
  val c = new C
  var vc = new C
  lazy val lz = { println("  lazy"); new C }
  val holder = new Holder
  val flags = new WithFlags
  val lv = true
  println("true"); println((if true then "" + l else "") + tail())
  println("object final val"); println((if Consts.T then "" + l else "") + tail())
  println("object inline val"); println((if Consts.I then "" + l else "") + tail())
  println("object val"); println((if Consts.V then "" + l else "") + tail())
  println("bare object final val"); println((if Bare.T then "" + l else "") + tail())
  println("other file's inline val"); println((if Other.J then "" + l else "") + tail())
  println("other file's final val"); println((if Other.F then "" + l else "") + tail())
  println("top-level final val"); println((if TopF then "" + l else "") + tail())
  println("top-level inline val"); println((if TopI then "" + l else "") + tail())
  println("own final val"); println(c.own(l))
  println("this.final val"); println(c.ownThis(l))
  println("final val behind a val"); println((if c.yes then "" + l else "") + tail())
  println("inline val behind a val"); println((if c.iyes then "" + l else "") + tail())
  println("val behind a val"); println((if c.plain then "" + l else "") + tail())
  println("final val behind a var"); println((if vc.yes then "" + l else "") + tail())
  println("final val behind a lazy val"); println((if lz.yes then "" + l else "") + tail())
  println("inline val behind a lazy val"); println((if lz.iyes then "" + l else "") + tail())
  println("final val behind a def"); println((if getC().yes then "" + l else "") + tail())
  println("final val behind new"); println((if new C().yes then "" + l else "") + tail())
  println("final val behind a path"); println((if holder.c.yes then "" + l else "") + tail())
  println("a trait's final val"); println((if flags.on then "" + l else "") + tail())
  println("local val"); println((if lv then "" + l else "") + tail())
  println("comparison"); println((if 1 < 2 then "" + l else "") + tail())
  println("negation"); println((if !false then "" + l else "") + tail())
  println("conjunction with a final val"); println((if Bare.T && true then "" + l else "") + tail())
  println("comparison of a final val"); println((if Bare.T == true then "" + l else "") + tail())
  println("arithmetic"); println((if 1 + 1 == 2 then "" + l else "") + tail())
  println("strings"); println((if "a" == "a" then "" + l else "") + tail())
  println("ascribed true"); println((if (true: Boolean) then "" + l else "") + tail())
  println("block of true"); println((if { true } then "" + l else "") + tail())
  println("block with a statement"); println((if { println("  cond"); true } then "" + l else "") + tail())
  println("conjunction with a call"); println((if true && { println("  cond"); true } then "" + l else "") + tail())
  println("disjunction short of a call"); println((if true || { println("  cond"); true } then "" + l else "") + tail())
  println("ascribed to an alias of Boolean"); println((if (true: Bool) then "" + l else "") + tail())
  println("negated ascription"); println((if !(false: Boolean) then "" + l else "") + tail())
