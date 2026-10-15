// Overloads over `collection` and `immutable` sequences and sets, whose lean std binds both names of one
// root to one class: a selection's signature keeps each parameter's own name, position by position, as
// `TreePickler` 487–508 writes the selected declaration's.
package sigprobe
object Lib:
  def mix(a: scala.collection.Seq[Int], b: scala.collection.immutable.Seq[Int], tag: Int): Int = a.size + b.size
  def choose(a: scala.collection.Seq[Int], tag: Int): Int = 1
  def choose(a: scala.collection.immutable.Seq[Int], tag: String): Int = 2
  def pick(a: scala.collection.Set[Int], tag: Int): Int = 3
  def pick(a: scala.collection.immutable.Set[Int], tag: String): Int = 4
  def mix(a: scala.collection.immutable.Seq[Int], b: scala.collection.Seq[Int], tag: String): Int = 99
