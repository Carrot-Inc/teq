object U12:
  def make(): BasicQueue = new BasicQueue with Incrementing12 with Filtering12

trait Incrementing12 extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 12)

trait Filtering12 extends Queue:
  abstract override def put(x: Int): Unit = if x >= 12 then super.put(x)
