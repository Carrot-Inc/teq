package meridian.web.css

import meridian.web.vdom.{AttrMod, Mod}

/** A class string that passed the validation at compile time. */
opaque type Tw = String
object Tw:
  def unsafe(value: String): Tw = value
  val empty: Tw = ""
  extension (tw: Tw)
    def text: String = tw
    def isEmpty: Boolean = tw.isEmpty
    def ++(that: Tw): Tw = concatClasses(List(tw, that))
    def whenTw(condition: Boolean): Tw = if condition then tw else ""

type ClassArg = String | (String, Boolean) | Tw | (Tw, Boolean)

/** `cls := (a, b -> cond, tw"...")`: every literal checked against the catalog next to the
  * sources, the pairs kept when their flag is true, and the result joined with one space. */
object cls:
  inline def :=(inline classes: ClassArg*): Mod = ${ CssMacros.clsImpl('classes) }
  def :=?(classes: Option[Tw]): Mod = classes match
    case Some(tw) => new AttrMod("class", tw.text)
    case None => new AttrMod("class", "")
  def raw(text: String): Mod = new AttrMod("class", text)

extension (inline sc: StringContext)
  inline def tw(inline args: Any*): Tw = ${ CssMacros.twImpl('sc, 'args) }

inline def classNames(inline classes: ClassArg*): Tw = ${ CssMacros.classListImpl('classes) }

def concatClasses(classes: Seq[ClassArg]): Tw =
  classes.toList.flatMap {
    case (text: String, flag: Boolean) => if flag then List(text) else Nil
    case text: String => List(text)
  }.filter(_.nonEmpty).mkString(" ")

def addClasses(classes: Seq[ClassArg]): Mod = new AttrMod("class", concatClasses(classes).text)
