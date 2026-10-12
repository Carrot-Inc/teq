package lta

// Local traits of upstream bodies whose members read the values of their block: the members take
// them as proxies and the classes mixing the trait in forward to them; the pickle writes the
// block as it is written.
trait Getter:
  def get: Int

trait Counter:
  def next(): Int

object Lib:
  def make(n: Int): Getter =
    trait T extends Getter:
      def get = n
    new T {}
  def counter(start: Int): Counter =
    var x = start
    trait T extends Counter:
      val step = { x += 0; 1 }
      def next(): Int = { x += step; x }
    class C extends T
    new C
  inline def twice(n: Int): Int =
    trait T:
      def get = n * 2
    class C extends T
    (new C).get
