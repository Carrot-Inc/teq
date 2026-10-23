package modref
class Out[+A]
object Obj
object ModuleRef:
  def id[A](a: A): A = a
  def wrap[A](a: A): Out[A] = ???
  val plain = Obj
  val single = id(Obj)
  val wrapped = wrap(Obj)
  val ascribed = id(Obj: Obj.type)
