// An object's inherited class exported through another object: its products name the class through
// the object's module (`TERMREF HtmlTagOf`), its bounds as seen from it (dotty's `pathType.select`).
package xinh
trait TagLite:
  class Tag[+N](val value: N)
object HtmlTagOf extends TagLite
object API:
  export HtmlTagOf.Tag
