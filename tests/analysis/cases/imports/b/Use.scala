package imb

// Imports inside a class body and a method body that nothing uses: the import alone depends
// on the upstream member.
class Use:
  import ima.Lib.n
  def f: Int = 0

class Use2:
  def g: Int =
    import ima.Lib.{m => mm, T}
    0

object Use3:
  import ima.Other.*
  def h: Int = 1
  def i: Int =
    import ima.Lib.Box as B
    2
