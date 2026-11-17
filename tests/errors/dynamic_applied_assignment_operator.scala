import scala.language.dynamics
class Holder(val n: Int):
  private var cell = 10
  def apply(i: Int): Int = cell
  def update(i: Int, v: Int): Unit = cell = v
class Typed extends Dynamic:
  val h = new Holder(0)
  def selectDynamic(name: String): Holder = h
  def applyDynamic(name: String)(args: Any*): Holder = h
class Selecting extends Dynamic:
  val h = new Holder(0)
  def selectDynamic(name: String): Holder = h
class Cell extends Dynamic:
  def applyDynamic(name: String)(args: Any*): Int = 5
def make(): Typed = new Typed
def uses(a: Typed, s: Selecting, c: Cell): Unit =
  a.slot(1) -= 2
  make().slot(1) -= 2
  s.slot(1) -= 2
  c("x") += 1
  c.slot(1) += 2
// An assignment operator on an applied left side of a value whose class derives from `scala.Dynamic`: the
// application as written is the dynamic method's (`a.applyDynamic("slot")(1)`, `c.applyDynamic("apply")("x")`),
// and where its result lacks the operator nothing else is tried: scalac makes no `update` of a dynamic
// application, and `a.slot` is no value `a.selectDynamic("slot")` to index, whatever `apply` and `update` that
// value would have. The error is the application's as written.
// expect: 17:3: error: value -= is not a member of Holder
// expect: 18:3: error: value -= is not a member of Holder
// expect: 19:3: error: value applyDynamic is not a member of Selecting
// expect: 20:3: error: value += is not a member of Int
// expect: 21:3: error: value += is not a member of Int
// expect: 5 errors found
