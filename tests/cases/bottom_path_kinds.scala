// `Checking.preCheckKind` keeps an argument whose widened type is a reference to `Nothing` before it compares kinds
// (`arg.tpe.widen.isRef(defn.NothingClass)`): `n.type` for `n: Nothing` (or of an alias of `Nothing`, or annotated) is
// an argument of any kind, in an applied type, an explicit constructor's arguments and a binder's annotation, as scalac
// 3.8.4 accepts it (the review of `6c316c160`, finding 2's BottomPath).
class Box[F[_]]
class ann[F[_]] extends scala.annotation.StaticAnnotation
type N = Nothing
object BottomPath:
  def f(n: Nothing): Box[n.type] = ???
  def g(n: Nothing): Unit =
    type L = [@ann[n.type] A] =>> Int
    ()
  def h(n: Nothing) = new Box[n.type]
  def i(n: N): Box[n.type] = ???
  def j(n: Nothing): Box[n.type @scala.annotation.internal.RuntimeChecked] = ???

@main def run(): Unit = println("ok")
