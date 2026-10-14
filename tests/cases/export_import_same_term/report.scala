// An object and a class imported by name and through a value whose class exports them bind once: a
// stable forwarder of an object is the object's path, a type forwarder the class (dotty's isSameRef;
// lila's `trans` and `Translate` beside `import helpers.*`).
package lreport
import lcore.i18n.{ I18nKey as trans, Translate }
final class ReportUi(helpers: lui.Helpers):
  import helpers.{ given, * }
  def page(using t: Translate): String = trans.site + " " + t.lang
@main def run(): Unit =
  given lcore.i18n.Translate = lcore.i18n.Translate("en")
  println(ReportUi(lui.Helpers()).page)
