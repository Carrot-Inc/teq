// What a class exports, its own clauses' and its parents', terms and types, comes with an import of
// a value of the class, wildcard or named, as a selection of the value reaches it: an export
// forwarder is a member.
package keys:
  final class Translate(val lang: String)
  object I18nKey:
    object search:
      val x = "search.x"
    val y = "y"
    def greet(n: String) = s"hello $n"

package ui:
  trait Helper:
    export keys.I18nKey as trans
    export keys.I18nKey.greet
    export keys.Translate
  class Helpers extends Helper

  class Wildcard(helpers: Helpers):
    import helpers.*
    import trans.search as trs
    def show = List(trs.x, trans.y, greet("w"))
    def lang(using t: Translate): String = t.lang

  class Named(helpers: Helpers):
    import helpers.{ trans, greet as hi }
    def show = List(trans.search.x, hi("n"))

  @main def run(): Unit =
    println(Wildcard(Helpers()).show)
    println(Wildcard(Helpers()).lang(using keys.Translate("fr")))
    println(Named(Helpers()).show)
