package seals

object Use5:
  def plain(s: Stamps): Unit = s.one(Seal[Int]())
  def overloaded(s: Stamps): Unit = s.two(5)(Seal[Int]())
  def overloaded2(s: Stamps): Unit = s.two(5, 2)(Seal[Int]())
  def curried(s: Stamps): Unit = s.curried(5)(2)(Seal[Int]())
  def ctor: Crate[Int] = new Crate(5)(Seal[Int]())
  def ext: Unit = Marks.stamped(5)(Seal[Int]())
  def old: Unit = Olds.old(5)(Seal[Int]())
