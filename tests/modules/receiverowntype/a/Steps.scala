package rota

// A product's value whose type an extension method inferred from a receiver of an alias of a union: its type
// keeps the alias, which a module built against the product reads.
type Step = "first" | "second"

extension [A](a: A)
  def some: Option[A] = Some(a)

object Steps:
  val step: Step = "first"
  val saved = step.some
  var stepVar: Step = "second"
  val savedVar = stepVar.some
  def stepDef: Step = "first"
  val savedDef = stepDef.some
