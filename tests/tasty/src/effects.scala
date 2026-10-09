// The effect a library is compiled against (the dummy `EffDefaults`, never shipped: its TASTy
// is left out of the fixtures), a JavaScript function as scalajs-react's is, and the library,
// whose bodies scalac pickles with its effect dealiased; tests/tasty/src-shadow/effects.scala
// is the `EffDefaults` programs link. Compiled with Scala.js (`--js`).
package fix.shadow

import scala.scalajs.js

trait Eff[F[_]]:
  def delay[A](a: => A): F[A]
  def run[A](fa: F[A]): A

trait EffApi:
  type F[A]
  implicit def F: Eff[F]

object EffDefaults extends EffApi:
  override type F[A] = js.Function0[A]
  override implicit val F: Eff[F] = new Eff[F]:
    def delay[A](a: => A): js.Function0[A] = () => a
    def run[A](fa: js.Function0[A]): A = fa()

final class EffRunner[G[_]](s: Eff[G]):
  def twice(x: Int): Int = s.run(s.delay(x * 2))

final class EffScope[F[_]](val n: Int)(using s: Eff[F]):
  def get: Int = s.run(s.delay(n + 1))

final class EffSlot[F[_]](val v: Option[F[Int]]):
  def size: Int = v.size

object EffSlot:
  def of[F[_]](s: Eff[F], n: Int): EffSlot[F] = EffSlot(Some(s.delay(n)))

import EffDefaults.*

object EffLib:
  def runTwice(x: Int): Int = new EffRunner(EffDefaults.F).twice(x)
  def use(s: EffScope[EffDefaults.F]): Int = s.get
  def scoped(n: Int): Int = use(new EffScope(n))
  def empty: EffSlot[EffDefaults.F] = new EffSlot(None)
  def slot(n: Int): EffSlot[EffDefaults.F] = EffSlot.of(EffDefaults.F, n)
  def mapped(n: Int): Int = EffSlot.of(EffDefaults.F, n).v.map(x => EffDefaults.F.run(x)).getOrElse(0)
  def sized(n: Int): Int = List(EffSlot.of(EffDefaults.F, n)).map(s => s.size).sum
  def runner = new EffRunner(EffDefaults.F)
  def viaRunner(x: Int): Int = runner.twice(x)
  def viaVal(n: Int): Int =
    val d = EffDefaults.F.delay(n)
    EffDefaults.F.run(d)
  def viaNestedVals(n: Int): Int =
    val a = { val b = { val c = { val d = EffDefaults.F.delay(n); d }; c }; b }
    EffDefaults.F.run(a)

final class EffCell[F[_], X](val x: X)(using s: Eff[F]):
  def get: X = s.run(s.delay(x))

object EffAliases:
  type Cell[X] = EffCell[EffDefaults.F, X]

object EffCells:
  import EffAliases.*
  def useCell(c: EffCell[EffDefaults.F, Int]): Int = c.get
  def cell(x: Int): Int = useCell(new Cell(x))

final case class EffLens[A, B](get: A => B)(val set: B => A => A):
  def mod(f: B => B): A => A = a => set(f(get(a)))(a)

final case class EffState[F[_]](slot: Option[Int])

object EffLenses:
  def slotL[F[_]] = EffLens((s: EffState[F]) => s.slot)(n => _.copy(slot = n))

object EffStates:
  type State = EffState[EffDefaults.F]
  def bump(s: State): State =
    val update: State => State = EffLenses.slotL.mod(_.map(_ + 1))
    update(s)
