// The given found for an extension's type variable follows the receiver: one written as a plain path to a term whose
// declared type is an alias of a union keeps the alias, and the given for it is the one found; an ascription to the
// alias, of a literal or of such a reference, or a literal, is widened to the members' class.
trait Label[A]:
  def text: String

object Steps:
  type Step = "first" | "second"
  given Label[String] with
    def text = "wide"
  given Label[Step] with
    def text = "narrow"
  extension [A](a: A)
    def label(using l: Label[A]): String = l.text
  val step: Step = "first"
  var stepVar: Step = "second"
  lazy val stepLazy: Step = "first"
  def stepDef: Step = "first"
  def stepPoly[T]: Step = "second"
  def byName(s: => Step): String = s.label

@main def run(): Unit =
  import Steps.{given, *}
  println(step.label)
  println(stepVar.label)
  println(stepLazy.label)
  println(stepDef.label)
  println(byName("second"))
  println(("first": Step).label)
  println("first".label)
  println((step: Step).label)
  println((stepVar: Step).label)
  println((stepDef: Step).label)
  // An annotation-only ascription of an unstable reference, an applied lambda, a polymorphic def given its type
  // argument and a cast are no references.
  println((stepVar: @unchecked).label)
  println((stepDef: @unchecked).label)
  println((stepVar: @scala.annotation.unchecked.uncheckedVariance).label)
  println((() => step)().label)
  println((() => stepVar)().label)
  println(stepPoly[Int].label)
  println(stepVar.asInstanceOf[Step].label)
