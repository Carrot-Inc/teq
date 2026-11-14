package seals

object Use7:
  def plain(s: Stamps): Unit = s.one(Seal[Int]())
  def overloaded(s: Stamps): Unit = s.two(7)(Seal[Int]())
  def overloaded2(s: Stamps): Unit = s.two(7, 2)(Seal[Int]())
  def curried(s: Stamps): Unit = s.curried(7)(2)(Seal[Int]())
  def ctor: Crate[Int] = new Crate(7)(Seal[Int]())
  def ext: Unit = Marks.stamped(7)(Seal[Int]())
  def old: Unit = Olds.old(7)(Seal[Int]())
