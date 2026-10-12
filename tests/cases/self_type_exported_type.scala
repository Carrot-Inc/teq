// A type a class of the self type exports is in scope, a member of the self type found through this
// (dotty's Typer.findRef): lila's FormHelper, self: I18nHelper & AssetHelper =>, naming Translate.
object i18n:
  final class Translate(val lang: String)
  type Lang = String

trait I18nHelper:
  export i18n.Translate
  type Key = String

trait AssetHelper:
  def asset: String = "a"

trait FormHelper:
  self: I18nHelper & AssetHelper =>
  def errMsg(k: Key)(using t: Translate): String = s"$k in ${t.lang} $asset"

object Helpers extends FormHelper, I18nHelper, AssetHelper

@main def run(): Unit =
  given i18n.Translate = i18n.Translate("en")
  println(Helpers.errMsg("x"))
