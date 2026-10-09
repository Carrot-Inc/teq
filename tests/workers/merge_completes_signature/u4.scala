object U4:
  def make(): BasicQueue = new BasicQueue with Incrementing4 with Filtering4

trait Incrementing4 extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 4)

trait Filtering4 extends Queue:
  abstract override def put(x: Int): Unit = if x >= 4 then super.put(x)
