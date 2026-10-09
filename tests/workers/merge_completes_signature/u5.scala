object U5:
  def make(): BasicQueue = new BasicQueue with Incrementing5 with Filtering5

trait Incrementing5 extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 5)

trait Filtering5 extends Queue:
  abstract override def put(x: Int): Unit = if x >= 5 then super.put(x)
