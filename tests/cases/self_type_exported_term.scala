// A term a class of the self type exports is in scope, a member of the self type found through this
// (dotty's Typer.findRef): lila's FlashHelper, self: I18nHelper =>, reading the trans I18nHelper exports.
object i18n:
  object I18nKey:
    object site:
      def success(): String = "Success"
  def tr(k: String): String = s"<$k>"

trait I18nHelper:
  export i18n.I18nKey as trans
  export i18n.tr

trait FlashHelper:
  self: I18nHelper =>
  def flash(msg: String): String = if msg.isEmpty then trans.site.success() else tr(msg)

object Helpers extends FlashHelper, I18nHelper

@main def run(): Unit =
  println(Helpers.flash(""))
  println(Helpers.flash("x"))
