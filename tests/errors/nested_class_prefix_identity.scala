// expect: 15:29: error: type mismatch: found (b.R : R), required (O.this.R : R)
// expect: 24:7: error: class C has conflicting base types a.I and b.I
// expect: 29:23: error: type mismatch: found o.Item, required o2.Item
// expect: 31:25: error: type mismatch: found o.Item, required w.Item
// expect: 33:26: error: type mismatch: found (joined : ((o : O) | (o2 : O))#Item), required o.Item
// expect: 35:22: error: type mismatch: found (p : O#Item), required o.Item
// expect: 6 errors found

// An instance's class is not another instance's (`TypeComparer.isSubPrefix`): through a val
// widened to the class, through the join of two instances, a nested object of the enclosing
// `this` against another instance's, a projection against a path, two prefixes of one trait.
class O:
  class Item
  object R
  def wrong(b: O): R.type = b.R

class P(val n: Int):
  trait I:
    def get = n
object H:
  val a = new P(1)
  val b = new P(2)
class A extends H.a.I
class C extends A with H.b.I

@main def run(): Unit =
  val o = new O
  val o2 = new O
  val path: o2.Item = new o.Item
  val w: O = o
  val widened: w.Item = new o.Item
  val joined = if path eq null then new o.Item else new o2.Item
  val narrowed: o.Item = joined
  val p: O#Item = new o.Item
  val back: o.Item = p
