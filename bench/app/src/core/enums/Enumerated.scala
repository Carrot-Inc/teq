package meridian.core.enums

/** The values of an enum whose cases are all singletons, with the name each travels under: the
  * enum's `label` member when it declares one, else the case name. Built by a macro. */
trait Enumerated[E]:
  def enumName: String
  def values: IArray[E]
  extension (e: E) def entryName: String
  def size: Int = values.length
  def valueList: List[E] = values.toList
  def valueOf(name: String): Option[E] = values.find(_.entryName == name)
  def valueOfIgnoreCase(name: String): Option[E] = values.find(_.entryName.equalsIgnoreCase(name))
  def fromOrdinal(i: Int): Option[E] = if i >= 0 && i < values.length then Some(values(i)) else None
  def ordinalOf(e: E): Int = values.indexOf(e)
  def next(e: E): E = values((ordinalOf(e) + 1) % values.length)

object Enumerated:
  transparent inline def derived[E]: Enumerated[E] = ${ EnumMacros.enumerated[E] }
  inline def apply[E](using e: Enumerated[E]): e.type = e

/** The travelling name of any enum case, singleton or class case, by the same rule. */
trait Labelled[E]:
  def enumName: String
  extension (e: E) def entryName: String

object Labelled:
  transparent inline def derived[E]: Labelled[E] = ${ EnumMacros.labelled[E] }
  inline def apply[E](using l: Labelled[E]): l.type = l
