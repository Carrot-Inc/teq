package csa

// A concrete var's setter is a member (dotty's `Desugar.isSetterNeeded`): selected by its name,
// an alternative of a written method of that name, through a trait, a constructor parameter, a
// top-level var and a generic class, and as a function value; its pickled body is `()`, which a
// downstream reads as the assignment (`Memoize`'s body). A private var has none.
object O:
  var n = 0
  def n_=(x: Short): Unit = n = 99

object S:
  var m = 0
  private var hidden = 1
  def bump(): Unit = hidden += 1
  def seen = hidden

trait T:
  var t = 1

class C(var c: Int) extends T

var top: Int = 0

class Box[A](init: A):
  var value: A = init
