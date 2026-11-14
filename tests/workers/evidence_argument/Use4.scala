package seals

object Use4:
  def plain(s: Stamps): Unit = s.one(Seal[Int]())
  def overloaded(s: Stamps): Unit = s.two(4)(Seal[Int]())
  def overloaded2(s: Stamps): Unit = s.two(4, 2)(Seal[Int]())
  def curried(s: Stamps): Unit = s.curried(4)(2)(Seal[Int]())
  def ctor: Crate[Int] = new Crate(4)(Seal[Int]())
  def ext: Unit = Marks.stamped(4)(Seal[Int]())
  def old: Unit = Olds.old(4)(Seal[Int]())
