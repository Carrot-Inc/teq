package seals

object Use8:
  def plain(s: Stamps): Unit = s.one(Seal[Int]())
  def overloaded(s: Stamps): Unit = s.two(8)(Seal[Int]())
  def overloaded2(s: Stamps): Unit = s.two(8, 2)(Seal[Int]())
  def curried(s: Stamps): Unit = s.curried(8)(2)(Seal[Int]())
  def ctor: Crate[Int] = new Crate(8)(Seal[Int]())
  def ext: Unit = Marks.stamped(8)(Seal[Int]())
  def old: Unit = Olds.old(8)(Seal[Int]())
