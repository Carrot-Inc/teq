// The consumers of a class named through a path inside a library module's own bodies, a method's and an inline
// method's: its ClassTag (alone, of a generic class, of an array of it, of a projection), its own `==`. A second
// module reads them from this one's products, expands the inline ones and has its own.
package lib

class Ops(val seed: Int):
  class Top(val v: Int)
  object nums:
    class Box(val v: Int) { def sum: Int = v + seed }
    class Cell[T](val get: T)
    class Money(val n: Int) { def ==(o: Money): Boolean = n == o.n; def !=(o: Money): Boolean = n != o.n }
  def tagged: Int = Array.empty[nums.Box].length + Array(new nums.Box(1), new nums.Box(2)).map(_.sum).sum
  def boxes(n: Int): Array[nums.Box] = Array.fill(n)(new nums.Box(n))
  def same: Boolean = new nums.Money(1) == new nums.Money(1)
  def differ: Boolean = new nums.Money(1) != new nums.Money(1)
  inline def fresh: Int = Array.empty[nums.Box].length
  inline def cells: Int = Array.empty[nums.Cell[Int]].length + Array(new nums.Cell(1)).length
  inline def nested: Int = Array.empty[Array[nums.Box]].length + Array(Array(new nums.Box(1))).length
  inline def projected: Int = Array.empty[Ops#Top].length + Array[Ops#Top](new Top(1)).length
  inline def inlined: Boolean = new nums.Money(1) == new nums.Money(1)
