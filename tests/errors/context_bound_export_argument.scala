// teq: --source 3.5
// An explicit argument for a context bound's evidence, passed to the method and through an export's forwarder:
// the forwarder's first parameter is named as the evidence is (dotty's `ContextBoundParamName`), so its call is
// reported as the method's own is (`Migrations.contextBoundParams`); a call that leaves the evidence to the
// search is not.
class Seal[T]
given [T]: Seal[T] = Seal[T]()

class Stamps:
  def one[X: Seal]: Int = 1

object Desk:
  val stamps = Stamps()
  export stamps.one

object Use:
  def direct: Int = Desk.stamps.one(Seal[Int]())
  def forwarded: Int = Desk.one(Seal[Int]())
  def searched: Int = Desk.one
  def searchedDirect: Int = Desk.stamps.one
// expect: 17:21: error: Context bounds will map to context parameters.
// expect: 18:24: error: Context bounds will map to context parameters.
// absent: context_bound_export_argument.scala:19:
// absent: context_bound_export_argument.scala:20:
