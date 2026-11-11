// A splice's type arguments are no dependency of the macro on the run's definitions: macroDependencies walks no
// type tree, so `Obj` of the same file, named only in `Obj.type` and `List[Obj.type]`, is not called by the macro.
object Obj:
  val v: Int = 1

object M:
  inline def singleton: String = ${ libmac.LibMac.expanded[Obj.type] }
  inline def applied: String = ${ libmac.LibMac.expanded[List[Obj.type]] }

object SameFile:
  def both: String = M.singleton + " " + M.applied
