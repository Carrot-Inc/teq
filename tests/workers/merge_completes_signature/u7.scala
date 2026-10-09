object U7:
  def make(): BasicQueue = new BasicQueue with Incrementing7 with Filtering7

trait Incrementing7 extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 7)

trait Filtering7 extends Queue:
  abstract override def put(x: Int): Unit = if x >= 7 then super.put(x)
