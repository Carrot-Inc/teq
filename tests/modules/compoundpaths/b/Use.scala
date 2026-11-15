package cpb

import cpa.*
import cpa.Hold.given

class Later(val d: Desk, val o: Outer):
  def viaLazy(): Int = { Hold.late.Bin.n += 1; Hold.late.Bin.n }
  def viaGiven(): Int = { Hold.holder.Bin.n += 2; Hold.holder.Bin.n }
  def viaSummon(): Int = { summon[Shelf].Bin.n += 3; summon[Shelf].Bin.n }
  def viaCaseObject(): Int = { d.Tag.n += 4; d.Tag.n }
  def viaCompanion(): Int = { o.In.n += 5; o.In.n }
  def viaEnumCompanion(): Int = { o.Mode.count += 6; o.Mode.count }
  def viaCall(): Int = { Hold.make().Bin.n += 7; Hold.make().Bin.n }
  def viaElement(): Int = { Hold.shelves(0).Bin.n += 8; Hold.shelves(0).Bin.n }
  def viaSetter(): Int = { Hold.late.Bin.level += 9; Hold.late.Bin.level }

// The index form, whose receiver and whose argument are bound as well: the object with `apply` and `update` and
// the object as a key, each through a lazy value and through a call.
class IndexLazy:
  def r(): Int = { Hold.late.Slots(0) += 11; Hold.late.Slots(0) }

class IndexNoPath:
  def r(): Int = { Hold.make().Slots(1) += 12; Hold.make().Slots(1) }

class ArgLazy:
  def r(): Int = { Hold.ledger(Hold.late.Bin) += 13; Hold.ledger(Hold.late.Bin) }

class ArgNoPath:
  def r(): Int = { Hold.ledger(Hold.make().Bin) += 14; Hold.ledger(Hold.make().Bin) }

object Use:
  def main(args: Array[String]): Unit =
    val l = new Later(new Desk, new Outer)
    println((l.viaLazy(), l.viaGiven(), l.viaSummon(), l.viaCaseObject(), l.viaCompanion(), l.viaEnumCompanion(), l.viaCall(), l.viaElement(), l.viaSetter(), Hold.late.own(1)))
    println((new IndexLazy().r(), new IndexNoPath().r(), new ArgLazy().r(), new ArgNoPath().r()))
