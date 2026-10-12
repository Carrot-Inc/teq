// expect: 14:30: error: extension method page is not applicable to o.Status
// expect: 1 error found
// An extension inherited from a trait is seen from the `this` of the class that finds it lexically
// (`Clas.this.page`, whose parameter is `Clas.this.Status`), which another instance's `Status`
// does not conform to; through that instance (`o.page`) it applies. scalac reports the mismatch
// at the argument (E007, `Found: (o.Ok : o.Status)`, `Required: Clas.this.Status`).
trait Results:
  class Status(val code: Int)
  val Ok = new Status(200)
trait CtrlPage extends Results:
  extension (s: Status) def page(p: String): String = p
class Clas extends CtrlPage:
  def good(o: Clas): String = o.page(o.Ok)("x") + o.Ok.page("y")
  def bad(o: Clas): String = page(o.Ok)("x")
