// expect: 28:33: error: Found:    Int & String
// expect: 29:33: error: Found:    (true : Boolean)
// expect: 30:38: error: Found:    Int & String & Double
// expect: 31:54: error: Found:    Int & String
// expect: 32:54: error: Found:    (true : Boolean)
// expect: 35:20: error: Found:    Int & String
// expect: 36:20: error: Found:    (true : Boolean)
// expect: 37:31: error: Found:    (true : Boolean)
// expect: 38:30: error: Found:    Int & String
// expect: 39:30: error: Found:    (true : Boolean)
// expect: 40:36: error: Found:    Int & String & Double
// expect: 46:42: error: Found:    (true : Boolean)
// expect: 12 errors found
// `argForParam`'s `AndOrType` arm (Types.scala 2802-2811) and `AsSeenFromMap.derivedSelect`'s range (6711-6712): a
// member of a base two or three instances give, seen from an intersection, a refinement over it, a self type, an
// overloaded member, a trait whose parents give the two, the trait and a third, and a given's; a result takes the
// range's upper end (`Int & String`), a parameter its lower (`Int | String`), as scalac 3.8.4 reports each. The
// closure adversary of the pass on 5bd85bcd1 (round 23).
trait Box[A]:
  def get: A
  def put(a: A): Int
  def m(a: A): Int
  def m(a: A, b: Int): Int
trait IB extends Box[Int]
trait SB extends Box[String]
trait DB extends Box[Double]
trait Both extends IB with SB
def a1(x: IB & SB): Boolean = x.get
def a2(x: IB & SB): Int = x.put(true)
def a3(x: IB & SB & DB): Boolean = x.get
def a4(x: (IB & SB) { def extra: Int }): Boolean = x.get
def a5(x: (IB & SB) { def extra: Int }): Int = x.put(true)
trait U6:
  self: IB & SB =>
  def g: Boolean = get
  def h: Int = put(true)
def a7(x: IB & SB): Int = x.m(true)
def a8(x: Both): Boolean = x.get
def a9(x: Both): Int = x.put(true)
def a10(x: Both & DB): Boolean = x.get
trait Show[A]:
  def show(a: A): String
trait IS extends Show[Int]
trait SS extends Show[String]
trait BothS extends IS with SS
def a11(using s: BothS): String = s.show(true)
// expect: Required: Boolean
// expect: Required: Int | String
// expect: Required: Boolean
// expect: Required: Boolean
// expect: Required: Int | String
// expect: Required: Boolean
// expect: Required: Int | String
// expect: Required: Int | String
// expect: Required: Boolean
// expect: Required: Int | String
// expect: Required: Boolean
// expect: Required: Int | String
