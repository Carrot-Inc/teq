package aar

// An anonymous class whose only parent is `AnyRef` or `Object`, which the pickle writes as its
// `Object()` parent (dotty's `TreeUnpickler.readParents` reads it as such): read back over the
// products it keeps that parent, as its source names it, beside one whose parents are a trait
// or `AnyRef` with a trait.
trait Named:
  def name: String

object Api:
  def make(): AnyRef = new AnyRef { override def toString = "anon" }
  def obj(): Object = new Object { override def toString = "obj" }
  def counted(): Int =
    var n = 0
    val o = new AnyRef { def bump(): Unit = n += 1; override def toString = s"n=$n" }
    o.toString.length + 20
  def named(): Named = new Named { def name = "named" }
  def both(): Named = new AnyRef with Named { def name = "both" }
