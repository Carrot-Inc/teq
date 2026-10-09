// A module's members read through its module symbol, as izumi-reflect finds `Tag.apply`:
// `requiredModule(..).declaredMethod(..)` and `declaredMethods` answer from the module's class.
package mmacro

import scala.quoted.*

object M:
  inline def show: String = ${ showImpl }
  def showImpl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val obj = Symbol.requiredModule("mm.Registry")
    val applies = obj.declaredMethod("apply").map(m => m.paramSymss.map(_.size).mkString("[", ",", "]"))
    val names = obj.declaredMethods.map(_.name).filter(n => !n.contains("$") && n != "writeReplace").sorted
    Expr(applies.sorted.mkString(" ") + " / " + names.mkString(","))
