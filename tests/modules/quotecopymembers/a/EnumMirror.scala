package ema

trait EnumMirror[E]:
  def mirroredName: String
  def size: Int
  def values: IArray[E]
  def valueOf(name: String): Option[E] = values.find(_.entryName == name)
  def fromOrdinal(ordinal: Int): Option[E] = if ordinal >= 0 && ordinal < size then Some(values(ordinal)) else None
  extension (e: E)
    def ordinal: Int
    def entryName: String

object EnumMirror:
  inline def apply[E](using mirror: EnumMirror[E]): mirror.type = mirror
  transparent inline def derived[E]: EnumMirror[E] = ${ Macros.derivedEnumMirror[E] }
