package seals

object Use1:
  def plain(s: Stamps): Unit = s.one(Seal[Int]())
  def overloaded(s: Stamps): Unit = s.two(1)(Seal[Int]())
  def overloaded2(s: Stamps): Unit = s.two(1, 2)(Seal[Int]())
  def curried(s: Stamps): Unit = s.curried(1)(2)(Seal[Int]())
  def ctor: Crate[Int] = new Crate(1)(Seal[Int]())
  def ext: Unit = Marks.stamped(1)(Seal[Int]())
  def old: Unit = Olds.old(1)(Seal[Int]())
