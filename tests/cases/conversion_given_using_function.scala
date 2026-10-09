// A given `Conversion[X, Step3[P, S]]` whose using clause `ev: X => Step2[P, S]` fixes `P` and `S`
// (scalajs-react's `Builder.defaultToNoBackend`): the receiver fixes `X` first, and the conversion's
// result has the `P` and `S` the evidence found, which a lambda argument's parameters take
// (scalac: "1/a", "1-1").
import scala.language.implicitConversions
object Builder:
  final class Step2[P, S](val p: P, val s: S)
  final class Step3[P, S](val step: Step2[P, S]):
    def show: String = s"${step.p}/${step.s}"
    def render(f: (P, S) => String): String = f(step.p, step.s)
  given toStep3[X, P, S](using ev: X => Step2[P, S]): Conversion[X, Step3[P, S]] = x => new Step3(ev(x))

@main def main(): Unit =
  val s2 = new Builder.Step2(1, "a")
  println(s2.show)
  println(s2.render((p, s) => s"$p-${s.length}"))
