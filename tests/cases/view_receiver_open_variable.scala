// A member reached through a given conversion whose own using clause fixes its type parameters
// (scalajs-react's builder: `initialState[S]` gives a `Step2[P, S]`, whose `renderPS` is `Step3`'s
// through `defaultToNoBackend[X, P, S](using ev: X => Step2[P, S])`). The receiver's `S` is held
// nonvariantly, so the receiver's interpolation leaves it open; `ev` makes it one with the
// instance's `S` (`ConstraintHandling.unify`). The view is the instance applied to the receiver,
// of that application's type (`Implicits.inferView`), `Step3[P, ?S]` with the enclosing `?S`
// open, and the lambda's parameter is defined from it (`state: State`), as scalac has it.
package viewreceiveropenvariable

import scala.language.implicitConversions

object Builder:
  final class Step1[P](private val name: String) extends AnyVal:
    def initialState[S](s: => S): Step2[P, S] = new Step2[P, S](name, () => s)
    def stateless: Step2[P, Unit] = new Step2[P, Unit](name, () => ())
  final class Step2[P, S](name: String, init: () => S):
    def noBackend: Step3[P, S, Unit] = new Step3[P, S, Unit](name, init)
  final class Step3[P, S, B](name: String, init: () => S):
    type $ = Scope[P, S, B]
    def renderPS(r: ($, P, S) => String): Last[P, S] = new Last[P, S](name, init, (p: P, s: S) => r(new Scope[P, S, B], p, s))
    def render_S(r: S => String): Last[P, S] = new Last[P, S](name, init, (_: P, s: S) => r(s))
  final class Scope[P, S, B]
  final class Last[P, S](val name: String, init: () => S, render: (P, S) => String):
    def show(p: P): String = s"$name: ${render(p, init())}"
  inline given defaultToNoState[P]: Conversion[Step1[P], Step2[P, Unit]] = _.stateless
  inline given defaultToNoBackend[X, P, S](using inline ev: X => Step2[P, S]): Conversion[X, Step3[P, S, Unit]] =
    ((x: X) => inline x match
      case y: Step2[P, S] => (y.noBackend: Step3[P, S, Unit])
      case _ => ev(x).noBackend
    ): Conversion[X, Step3[P, S, Unit]]

object EntryPoint:
  inline def apply[Props]: Builder.Step1[Props] = apply[Props]("auto")
  inline def apply[Props](inline displayName: String): Builder.Step1[Props] = new Builder.Step1[Props](displayName)
object Components:
  val builder: EntryPoint.type = EntryPoint

// The same through an `implicit def` conversion, whose result is its own solved variables'.
object DefBuilder:
  final class Start[P]:
    def initialState[S](s: => S): Middle[P, S] = new Middle[P, S](() => s)
  final class Middle[P, S](init: () => S):
    def finish: End[P, S] = new End[P, S](init)
  final class End[P, S](init: () => S):
    def renderS(r: S => String): String = r(init())
  implicit def toEnd[X, P, S](x: X)(using ev: X => Middle[P, S]): End[P, S] = ev(x).finish

final case class State(count: Option[Int], label: Option[String])
object State:
  val initial: State = State(Some(1), Some("two"))
final case class Props(id: Int)

object Main:
  // No expected type: the chain's result is the receiver of what follows.
  val root = Components
    .builder[Props]
    .initialState(State.initial)
    .renderPS { (scope, props, state) =>
      (state.count, state.label) match
        case (Some(a), Some(b)) => s"$a $b ${props.id}"
        case _                  => "empty"
    }
  val named = Components.builder[Props]("named").initialState(State(None, Some("x"))).render_S(state => state.label.getOrElse("-"))
  val bare = new Builder.Step1[Int]("bare").initialState(List(State.initial)).renderPS((_, n, states) => states.map(_.count.sum + n).mkString(","))
  val noState = Components.builder[Props]("stateless").renderPS((_, props, unit) => s"${props.id} $unit")
  val viaDef = new DefBuilder.Start[Int].initialState(State.initial).renderS(state => state.label.getOrElse("-"))
  def main(args: Array[String]): Unit =
    println(root.show(Props(3)))
    println(named.show(Props(4)))
    println(bare.show(5))
    println(noState.show(Props(6)))
    println(s"def: $viaDef")
