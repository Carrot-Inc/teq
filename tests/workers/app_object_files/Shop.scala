package market

// The program's entry: its body is the object's initialiser, and the `main` it inherits runs the block it
// gave `delayedInit`.
object Shop extends Stall:
  println("start")
  val goods: Int = Ledger.opened + 4
  delayedInit { println("block " + goods + " " + args.length) }
  println(goods)
