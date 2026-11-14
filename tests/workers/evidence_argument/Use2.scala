package seals

object Use2:
  def plain(s: Stamps): Unit = s.one(Seal[Int]())
  def overloaded(s: Stamps): Unit = s.two(2)(Seal[Int]())
  def overloaded2(s: Stamps): Unit = s.two(2, 2)(Seal[Int]())
  def curried(s: Stamps): Unit = s.curried(2)(2)(Seal[Int]())
  def ctor: Crate[Int] = new Crate(2)(Seal[Int]())
  def ext: Unit = Marks.stamped(2)(Seal[Int]())
  def old: Unit = Olds.old(2)(Seal[Int]())
