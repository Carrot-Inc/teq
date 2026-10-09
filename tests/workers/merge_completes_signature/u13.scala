object U13:
  def make(): BasicQueue = new BasicQueue with Incrementing13 with Filtering13

trait Incrementing13 extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 13)

trait Filtering13 extends Queue:
  abstract override def put(x: Int): Unit = if x >= 13 then super.put(x)
