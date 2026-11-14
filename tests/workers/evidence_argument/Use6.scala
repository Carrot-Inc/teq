package seals

object Use6:
  def plain(s: Stamps): Unit = s.one(Seal[Int]())
  def overloaded(s: Stamps): Unit = s.two(6)(Seal[Int]())
  def overloaded2(s: Stamps): Unit = s.two(6, 2)(Seal[Int]())
  def curried(s: Stamps): Unit = s.curried(6)(2)(Seal[Int]())
  def ctor: Crate[Int] = new Crate(6)(Seal[Int]())
  def ext: Unit = Marks.stamped(6)(Seal[Int]())
  def old: Unit = Olds.old(6)(Seal[Int]())
