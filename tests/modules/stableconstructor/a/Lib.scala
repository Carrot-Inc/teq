// A constructor is stable where its class runs nothing when an instance is made: a trait that is NoInits, a class
// that isNoInitsRealClass (`setStableConstructor`, SymDenotations.scala:97-99). A downstream reads the flag from the
// pickle: scalac's `isPureApply` (TreeInfo.scala:673) takes the creation of a stable constructor's class for pure,
// so its inliner drops an unused binding of it and its warning names it a pure expression in statement position.
package sca

class P(x: Int):
  print("P".concat(x.toString).concat(" "))

class P2(val x: Int):
  print("P2 ")

class S(x: Int):
  val y = x + 1

class R(x: Int)

class N(val n: Int)

trait TI:
  print("TI ")

trait TN:
  def f: Int = 1

class UsesTI extends TI

class UsesTN extends TN

case class K(n: Int):
  print("K ")

object Made:
  var count = 0
  def made(): Int = { count += 1; count }

class M(x: Int):
  val z = Made.made()

// An extension group and an export clause each initialise (`defKind` gives `NoInitsInterface` to an `Import`
// alone).
class Ext:
  extension (x: Int) def foo: Int = x + 1

class Box(val a: Int)

class Exporter(val b: Box):
  export b.a

object Src:
  val box = Box(9)

trait TExporter:
  export Src.box

class UsesTExporter extends TExporter
