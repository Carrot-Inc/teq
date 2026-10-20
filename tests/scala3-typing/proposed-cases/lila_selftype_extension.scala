// An extension method declared in the self type of a trait (lila's
// `trait StringHelper: self: I18nHelper & NumberHelper =>` calling `n.localize`).
trait NumberHelper:
  extension (e: Int) def localize: String = s"<$e>"

trait StringHelper:
  self: NumberHelper =>
  def pluralizeLocalize(s: String, n: Int) = s"${n.localize} $s"

object Helpers extends StringHelper with NumberHelper

@main def run() =
  println(Helpers.pluralizeLocalize("games", 3))
