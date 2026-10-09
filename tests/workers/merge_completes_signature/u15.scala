object U15:
  def make(): BasicQueue = new BasicQueue with Incrementing15 with Filtering15

trait Incrementing15 extends Queue:
  abstract override def put(x: Int): Unit = super.put(x + 15)

trait Filtering15 extends Queue:
  abstract override def put(x: Int): Unit = if x >= 15 then super.put(x)
