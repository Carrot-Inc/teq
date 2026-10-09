object U11:
  def make(): BasicQueue = new BasicQueue with Incrementing11 with Filtering11

trait Incrementing11 extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 11)

trait Filtering11 extends Queue:
  abstract override def put(x: Int): Unit = if x >= 11 then super.put(x)
