// A fully qualified use is no use of the import; a local definition of the name is not either.
object Lib { object Inner { val deep = 1 }; val x = 1 }
object Use {
  import Lib.Inner
  def f = Lib.Inner.deep
}
object Local {
  import Lib.x
  def g = { val x = 2; x }
}
