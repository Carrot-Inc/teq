package seals

object Use3:
  def plain(s: Stamps): Unit = s.one(Seal[Int]())
  def overloaded(s: Stamps): Unit = s.two(3)(Seal[Int]())
  def overloaded2(s: Stamps): Unit = s.two(3, 2)(Seal[Int]())
  def curried(s: Stamps): Unit = s.curried(3)(2)(Seal[Int]())
  def ctor: Crate[Int] = new Crate(3)(Seal[Int]())
  def ext: Unit = Marks.stamped(3)(Seal[Int]())
  def old: Unit = Olds.old(3)(Seal[Int]())
