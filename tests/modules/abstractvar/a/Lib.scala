package avl

// Abstract vars and a setter pair declared upstream: a downstream implements them with vars and
// with def pairs and assigns them through the upstream's types, which call the setters the
// upstream's pickles and class files declare, as do the upstream's own bodies.
trait AbsVar:
  var x: Int

trait DefPair:
  def y: Int
  def y_=(v: Int): Unit

trait Handle[A]:
  var current: A

abstract class Cell:
  var v: Int
  def set(n: Int): Unit = v = n
  def bump(): Unit = v += 1

class Box(var x: Int) extends AbsVar

// A setter overloaded beside the one a var implements.
abstract class Over:
  def w: Int
  def w_=(v: Int): Unit
  def w_=(v: String): Unit = w = v.length

// A var parameter of no abstract var, whose pickled setter has no body either.
class Counter(var n: Int):
  def next: Int = n + 1

object Lib:
  def assign(a: AbsVar, v: Int): Int = { a.x = v; a.x }
  def bump(a: AbsVar): Unit = a.x += 1
  def put[A](h: Handle[A], v: A): A = { h.current = v; h.current }
