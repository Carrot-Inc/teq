package emb

import ema.{EnumMirror, Pair, Shapes}

// No code of this module calls `ordinal` or `dead`: the later module does.
object Models:
  enum Color derives EnumMirror:
    case Red, Green, Blue

object Made:
  val anon: Pair = Shapes.anon
  val local: Pair = Shapes.local
  val solo: Pair = Shapes.solo
  val given_ : Pair = Shapes.given_
  val nested: Pair = Shapes.nested
  val kept: Class[?] = Shapes.kept
  val unused: Pair = Shapes.unused
  val outer: AnyRef = Shapes.outer
  val one: Int = Shapes.discarded
  val holder: Pair = Shapes.holder
  val lister: AnyRef = Shapes.lister
  val lifted: AnyRef = Shapes.lifted

  def touch: String =
    import Models.Color
    val m = summon[EnumMirror[Color]]
    s"${m.size} ${m.entryName(Color.Blue)} ${m.valueOf("Red")} ${m.fromOrdinal(2)} " +
      s"${anon.live(0)} ${local.live(0)} ${solo.live(0)} ${given_.live(0)} ${nested.live(0)} ${unused.live(0)} $one ${holder.live(0)}"
